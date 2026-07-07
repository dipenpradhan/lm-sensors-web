//! # Sensor layer
//!
//! Wraps the `lm-sensors` Rust crate to provide a safe, cloneable interface
//! for reading hardware sensor data (temperatures, voltages, fan speeds, etc.).
//!
//! # Architecture
//!
//! ```text
//! SensorManager
//! └── Arc<RwLock<SafeLMSensors>>
//!     └── lm_sensors::LMSensors  (unsafe impl Send + Sync)
//! ```
//!
//! The `lm-sensors` crate's `LMSensors` is not `Send + Sync` by default
//! because it holds pointers into libhardware-sensors C library state.
//! We wrap it in `SafeLMSensors` which is safe because:
//! - We only ever read (never write) through the lock
//! - `chip_iter()` returns immutable references
//! - No C library mutation occurs during iteration
//!
//! This allows sharing via `Arc<RwLock<SafeLMSensors>>` across tokio threads.
//!
//! # Testing
//!
//! Unit tests cover the data types (serialisation, round-trips, edge cases).
//! `SensorManager` tests require a live `libsensors` environment and are
//! integration-level — see `tests/sensor_manager.rs`.

use std::sync::{Arc, RwLock};

/// Re-export all data types so callers import from the parent module.
pub use self::data::*;

/// ── Data types (serialisation schema) ──────────────────────────────

/// Data types for sensor readings. These structs form the JSON schema
/// exposed via REST API (`/api/sensors`) and WebSocket broadcasts.
mod data {
    use serde::{Deserialize, Serialize};

    /// A single hardware sensor device (e.g. "coretemp-isa-0000").
    ///
    /// Each device represents a physical or virtual sensor reported by
    /// `lm-sensors`. Contains metadata but not readings.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Device {
        /// Device name (e.g. "coretemp", "nvme", "acpitz").
        pub name: String,
        /// Bus type (e.g. "ISA", "SMBus", "PCI").
        pub bus: String,
        /// Optional sysfs path (e.g. "/sys/class/hwmon/hwmon0").
        pub path: Option<String>,
    }

    /// A logical feature group on a device.
    ///
    /// A feature groups related sub-features. E.g. "temp1" groups
    /// `temp1_input`, `temp1_min`, `temp1_max`, `temp1_crit`.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FeatureInfo {
        /// Feature name (e.g. "temp1", "fan1", "in1").
        pub name: String,
        /// Sub-features under this feature (the actual sensor readings).
        pub sub_features: Vec<SubFeatureInfo>,
    }

    /// A single measurable sensor value.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SubFeatureInfo {
        /// Sub-feature name (e.g. "temp1_input").
        pub name: String,
        /// Numeric value (e.g. 65.0 for 65°C). `None` if unreadable.
        pub value: Option<f64>,
        /// Unit string (e.g. "°C", "V", "RPM").
        pub unit: Option<String>,
    }

    /// Top-level container for all sensor readings.
    ///
    /// This is the JSON payload sent over REST and WebSocket:
    /// ```json
    /// {"devices": [{"device": {...}, "features": [...]}, ...]}
    /// ```
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SensorReadings {
        /// All devices with their readings.
        pub devices: Vec<DeviceReadings>,
    }

    /// A single device paired with all its feature readings.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DeviceReadings {
        /// Device metadata (name, bus, path).
        pub device: Device,
        /// All features and sub-features with current values.
        pub features: Vec<FeatureInfo>,
    }
}

/// Send+Sync wrapper around `lm_sensors::LMSensors`.
///
/// The underlying `LMSensors` type is not `Send + Sync` because it wraps
/// C library state from libhardware-sensors. However, our usage is safe:
/// - We only ever read (never write) through the `RwLock` read guard
/// - `chip_iter()` returns immutable references, never mutates C state
///
/// This allows sharing via `Arc<RwLock<SafeLMSensors>>` across async tasks.
struct SafeLMSensors(lm_sensors::LMSensors);
unsafe impl Send for SafeLMSensors {}
unsafe impl Sync for SafeLMSensors {}

/// Thread-safe, cloneable sensor manager.
///
/// Wraps `lm-sensors` in `Arc<RwLock<>>` so multiple async tasks can
/// read sensor data concurrently without re-initializing the library.
/// Cheap to clone — only copies the `Arc`.
pub struct SensorManager {
    sensors: Arc<RwLock<SafeLMSensors>>,
}

impl Clone for SensorManager {
    fn clone(&self) -> Self {
        // Cheap clone — increments Arc refcount, no deep copy.
        Self {
            sensors: Arc::clone(&self.sensors),
        }
    }
}

unsafe impl Send for SensorManager {}
unsafe impl Sync for SensorManager {}

impl SensorManager {
    /// Initialize the sensor manager by opening the `lm-sensors` library.
    ///
    /// Returns an error if libsensors cannot be initialized (e.g. missing
    /// system library or no hardware sensors detected).
    pub fn new() -> Result<Self, String> {
        let sensors = lm_sensors::Initializer::default()
            .initialize()
            .map_err(|e| format!("Failed to initialize lm-sensors: {}", e))?;
        Ok(Self {
            sensors: Arc::new(RwLock::new(SafeLMSensors(sensors))),
        })
    }

    /// Execute a closure with read access to the underlying `LMSensors`.
    ///
    /// Acquires the `RwLock` read guard and passes the `LMSensors` reference
    /// to the closure. The guard is released when the closure returns.
    fn with_lock<T, F: FnOnce(&lm_sensors::LMSensors) -> T>(&self, f: F) -> T {
        let s = self.sensors.read().unwrap();
        f(&s.0)
    }

    /// List all detected sensor devices (metadata only, no readings).
    ///
    /// Useful for `/api/devices` — returns names, buses, and paths
    /// without the overhead of reading all feature values.
    pub fn list_devices(&self) -> Vec<Device> {
        self.with_lock(|s| s.chip_iter(None).map(|c| device_info(&c)).collect())
    }

    /// Look up a device by name (partial match).
    ///
    /// Searches all devices and returns the first whose name contains
    /// the given `name` substring. Case-sensitive.
    pub fn get_device(&self, name: &str) -> Option<Device> {
        self.with_lock(|s| {
            s.chip_iter(None)
                .find(|c| c.name().map_or(false, |n| n.contains(name)))
                .map(|c| device_info(&c))
        })
    }

    /// Look up a device by name and return it with all feature readings.
    ///
    /// Like `get_device()` but also reads every feature/sub-feature value.
    /// Used by `/api/devices/{id}/features`.
    pub fn get_device_features(&self, name: &str) -> Option<DeviceReadings> {
        self.with_lock(|s| {
            s.chip_iter(None)
                .find(|c| c.name().map_or(false, |n| n.contains(name)))
                .map(|c| DeviceReadings {
                    device: device_info(&c),
                    features: device_features(&c),
                })
        })
    }

    /// Read all sensor data from all devices.
    ///
    /// Primary method used by the WebSocket broadcast loop and
    /// the `/api/sensors` REST endpoint. Returns a complete snapshot
    /// of all devices, features, and current readings.
    pub fn read_all(&self) -> SensorReadings {
        self.with_lock(|s| SensorReadings {
            devices: s
                .chip_iter(None)
                .map(|c| DeviceReadings {
                    device: device_info(&c),
                    features: device_features(&c),
                })
                .collect(),
        })
    }
}

/// Extract basic device info from a `ChipRef`.
fn device_info(c: &lm_sensors::ChipRef) -> Device {
    Device {
        name: c.name().unwrap_or_default().to_string(),
        bus: c.bus().to_string(),
        // Convert OsStr → str, drop if not valid UTF-8.
        path: c.path().and_then(|p| p.to_str().map(str::to_string)),
    }
}

/// Extract all features and sub-features from a `ChipRef`.
///
/// Walks each feature and its sub-features, extracting readable values
/// and their units. Skips sub-features that fail to read (e.g. write-only).
fn device_features(c: &lm_sensors::ChipRef) -> Vec<FeatureInfo> {
    c.feature_iter()
        .filter_map(|f| {
            // Feature name may be None for anonymous features; use empty string.
            let name = f
                .name()
                .transpose()
                .ok()
                .flatten()
                .unwrap_or_default()
                .to_string();
            let subs: Vec<_> = f
                .sub_feature_iter()
                .filter_map(|sf| {
                    // Only include sub-features whose value can be read.
                    match sf.value() {
                        Ok(v) => Some(SubFeatureInfo {
                            // Format the value kind (e.g. "TempInput").
                            name: format!("{:?}", v.kind()),
                            value: Some(v.raw_value()),
                            unit: Some(unit_str(&v)),
                        }),
                        Err(_) => None, // Skip unreadable (write-only, etc.)
                    }
                })
                .collect();
            Some(FeatureInfo {
                name,
                sub_features: subs,
            })
        })
        .collect()
}

/// Convert an `lm_sensors::value::Unit` enum to a human-readable string.
fn unit_str(v: &lm_sensors::Value) -> String {
    let unit = v.unit();
    // Map the enum to common unit abbreviations for display.
    match unit {
        lm_sensors::value::Unit::Celcius => "°C",
        lm_sensors::value::Unit::Volt => "V",
        lm_sensors::value::Unit::Amp => "A",
        lm_sensors::value::Unit::Watt => "W",
        lm_sensors::value::Unit::Joule => "J",
        lm_sensors::value::Unit::RotationPerMinute => "RPM",
        lm_sensors::value::Unit::Percentage => "%",
        lm_sensors::value::Unit::Second => "s",
        lm_sensors::value::Unit::None => "",
        _ => "",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::data::*;

    // ── SensorReadings ──────────────────────────────────────────────

    /// Verify SensorReadings serialises to JSON with correct keys.
    #[test]
    fn test_readings_serde() {
        let r = SensorReadings {
            devices: vec![DeviceReadings {
                device: Device {
                    name: "test".into(),
                    bus: "ISA".into(),
                    path: Some("/sys/class/hwmon/hwmon0".into()),
                },
                features: vec![FeatureInfo {
                    name: "temp1".into(),
                    sub_features: vec![SubFeatureInfo {
                        name: "temp1_input".into(),
                        value: Some(65.0),
                        unit: Some("°C".into()),
                    }],
                }],
            }],
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("test"));
        assert!(j.contains("65.0"));
        assert!(j.contains("devices"));
    }

    /// Empty readings still produce valid JSON with "devices" key.
    #[test]
    fn test_empty_readings() {
        let r = SensorReadings { devices: vec![] };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("devices"));
    }

    /// Multiple devices with multiple features serialise correctly.
    #[test]
    fn test_multiple_devices_serialization() {
        let r = SensorReadings {
            devices: vec![
                DeviceReadings {
                    device: Device {
                        name: "cpu".into(),
                        bus: "ISA".into(),
                        path: None,
                    },
                    features: vec![FeatureInfo {
                        name: "temp1".into(),
                        sub_features: vec![SubFeatureInfo {
                            name: "temp1_input".into(),
                            value: Some(45.0),
                            unit: Some("°C".into()),
                        }],
                    }],
                },
                DeviceReadings {
                    device: Device {
                        name: "gpu".into(),
                        bus: "PCI".into(),
                        path: None,
                    },
                    features: vec![FeatureInfo {
                        name: "fan1".into(),
                        sub_features: vec![SubFeatureInfo {
                            name: "fan1_input".into(),
                            value: Some(1200.0),
                            unit: Some("RPM".into()),
                        }],
                    }],
                },
            ],
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("cpu"));
        assert!(j.contains("gpu"));
        assert!(j.contains("45.0"));
        assert!(j.contains("1200.0"));
        assert!(j.contains("RPM"));
    }

    /// SensorReadings round-trip through JSON preserves all fields.
    #[test]
    fn test_readings_serde_roundtrip() {
        let original = SensorReadings {
            devices: vec![DeviceReadings {
                device: Device {
                    name: "coretemp".into(),
                    bus: "ISA".into(),
                    path: Some("/sys/class/hwmon/hwmon0".into()),
                },
                features: vec![FeatureInfo {
                    name: "temp1".into(),
                    sub_features: vec![SubFeatureInfo {
                        name: "temp1_input".into(),
                        value: Some(55.5),
                        unit: Some("°C".into()),
                    }],
                }],
            }],
        };
        let j = serde_json::to_string(&original).unwrap();
        let parsed: SensorReadings = serde_json::from_str(&j).unwrap();
        assert_eq!(original.devices.len(), parsed.devices.len());
        assert_eq!(
            original.devices[0].device.name,
            parsed.devices[0].device.name
        );
        assert_eq!(
            original.devices[0].features[0].sub_features[0].value,
            parsed.devices[0].features[0].sub_features[0].value,
        );
    }

    // ── Device ──────────────────────────────────────────────────────

    /// Device with optional path serialises to JSON correctly.
    #[test]
    fn test_device_with_path() {
        let d = Device {
            name: "test".into(),
            bus: "ISA".into(),
            path: Some("/sys/class/hwmon/hwmon0".into()),
        };
        let j = serde_json::to_string(&d).unwrap();
        assert!(j.contains("test"));
        assert!(j.contains("ISA"));
        assert!(j.contains("hwmon0"));
    }

    /// Device with missing path serialises to null.
    #[test]
    fn test_device_without_path() {
        let d = Device {
            name: "test".into(),
            bus: "ISA".into(),
            path: None,
        };
        let j = serde_json::to_string(&d).unwrap();
        assert!(j.contains("null"));
    }

    /// Clone produces independent copies.
    #[test]
    fn test_device_clone() {
        let d = Device {
            name: "cpu".into(),
            bus: "ISA".into(),
            path: Some("/sys/hwmon0".into()),
        };
        let c = d.clone();
        assert_eq!(d.name, c.name);
        assert_eq!(d.bus, c.bus);
        assert_eq!(d.path, c.path);
    }

    // ── FeatureInfo ────────────────────────────────────────────────

    /// FeatureInfo with empty sub_features serialises correctly.
    #[test]
    fn test_feature_empty_subfeatures() {
        let f = FeatureInfo {
            name: "test".into(),
            sub_features: vec![],
        };
        let j = serde_json::to_string(&f).unwrap();
        assert!(j.contains("test"));
    }

    // ── SubFeatureInfo ─────────────────────────────────────────────

    /// SubFeatureInfo with None value and None unit serialises correctly.
    #[test]
    fn test_subfeature_none_values() {
        let s = SubFeatureInfo {
            name: "test".into(),
            value: None,
            unit: None,
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("test"));
        assert!(j.contains("null"));
    }

    // ── Debug formatting ───────────────────────────────────────────

    /// Debug formatting works for all data types.
    #[test]
    fn test_debug_formatting() {
        let r = SensorReadings {
            devices: vec![DeviceReadings {
                device: Device {
                    name: "test".into(),
                    bus: "ISA".into(),
                    path: None,
                },
                features: vec![FeatureInfo {
                    name: "temp1".into(),
                    sub_features: vec![SubFeatureInfo {
                        name: "temp1_input".into(),
                        value: Some(65.0),
                        unit: Some("°C".into()),
                    }],
                }],
            }],
        };
        let s = format!("{:?}", r);
        assert!(!s.is_empty());
    }
}

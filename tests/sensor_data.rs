//! # Sensor data model integration tests
//!
//! Tests the SensorReadings data model comprehensively, including edge cases,
//! serialization/deserialization, and structural integrity.
//!
//! # Running
//!
//! ```bash
//! cargo test --test sensor_data
//! ```

use lm_sensors_web::sensors::{
    Device, DeviceReadings, FeatureInfo, SensorReadings, SubFeatureInfo,
};

// ── Structural Tests ──────────────────────────────────────────────

/// Empty SensorReadings is valid.
#[test]
fn test_empty_sensor_readings() {
    let readings = SensorReadings { devices: vec![] };
    let json = serde_json::to_string(&readings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["devices"].is_array());
    assert_eq!(parsed["devices"].as_array().unwrap().len(), 0);
}

/// Single device with single feature.
#[test]
fn test_single_device_single_feature() {
    let readings = SensorReadings {
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
    let json = serde_json::to_string(&readings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["devices"][0]["device"]["name"], "test");
    assert_eq!(parsed["devices"][0]["features"][0]["name"], "temp1");
    assert_eq!(parsed["devices"][0]["features"][0]["sub_features"][0]["value"], 65.0);
}

/// Multiple devices with mixed feature types.
#[test]
fn test_mixed_feature_types() {
    let readings = SensorReadings {
        devices: vec![
            DeviceReadings {
                device: Device { name: "cpu".into(), bus: "ISA".into(), path: None },
                features: vec![
                    FeatureInfo {
                        name: "temp1".into(),
                        sub_features: vec![SubFeatureInfo {
                            name: "temp1_input".into(),
                            value: Some(45.0),
                            unit: Some("°C".into()),
                        }],
                    },
                    FeatureInfo {
                        name: "fan1".into(),
                        sub_features: vec![SubFeatureInfo {
                            name: "fan1_input".into(),
                            value: Some(1200.0),
                            unit: Some("RPM".into()),
                        }],
                    },
                ],
            },
            DeviceReadings {
                device: Device { name: "battery".into(), bus: "SMBus".into(), path: None },
                features: vec![FeatureInfo {
                    name: "in0".into(),
                    sub_features: vec![SubFeatureInfo {
                        name: "in0_input".into(),
                        value: Some(12.45),
                        unit: Some("V".into()),
                    }],
                }],
            },
        ],
    };
    let json = serde_json::to_string(&readings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["devices"].as_array().unwrap().len(), 2);
}

/// Device with empty features array.
#[test]
fn test_device_empty_features() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device { name: "empty".into(), bus: "ISA".into(), path: None },
            features: vec![],
        }],
    };
    let json = serde_json::to_string(&readings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["devices"][0]["features"].is_array());
    assert_eq!(parsed["devices"][0]["features"].as_array().unwrap().len(), 0);
}

/// SubFeature with None value (unreadable sensor).
#[test]
fn test_subfeature_none_value() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device { name: "test".into(), bus: "ISA".into(), path: None },
            features: vec![FeatureInfo {
                name: "temp1".into(),
                sub_features: vec![
                    SubFeatureInfo {
                        name: "temp1_input".into(),
                        value: Some(65.0),
                        unit: Some("°C".into()),
                    },
                    SubFeatureInfo {
                        name: "temp1_crit".into(),
                        value: None, // Read-only threshold
                        unit: Some("°C".into()),
                    },
                ],
            }],
        }],
    };
    let json = serde_json::to_string(&readings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["devices"][0]["features"][0]["sub_features"][0]["value"], 65.0);
    assert!(parsed["devices"][0]["features"][0]["sub_features"][1]["value"].is_null());
}

// ── Serialization Round-Trip Tests ──────────────────────────────

/// Full round-trip: serialize → deserialize → compare.
#[test]
fn test_full_roundtrip() {
    let original = SensorReadings {
        devices: vec![
            DeviceReadings {
                device: Device {
                    name: "coretemp-isa-0000".into(),
                    bus: "ISA".into(),
                    path: Some("/sys/class/hwmon/hwmon0".into()),
                },
                features: vec![FeatureInfo {
                    name: "temp1".into(),
                    sub_features: vec![
                        SubFeatureInfo {
                            name: "temp1_input".into(),
                            value: Some(55.0),
                            unit: Some("°C".into()),
                        },
                        SubFeatureInfo {
                            name: "temp1_max".into(),
                            value: Some(100.0),
                            unit: Some("°C".into()),
                        },
                    ],
                }],
            },
            DeviceReadings {
                device: Device {
                    name: "it8792-isa-0290".into(),
                    bus: "ISA".into(),
                    path: None,
                },
                features: vec![FeatureInfo {
                    name: "fan1".into(),
                    sub_features: vec![SubFeatureInfo {
                        name: "fan1_input".into(),
                        value: Some(900.0),
                        unit: Some("RPM".into()),
                    }],
                }],
            },
        ],
    };
    let json = serde_json::to_string(&original).unwrap();
    let parsed: SensorReadings = serde_json::from_str(&json).unwrap();

    assert_eq!(original.devices.len(), parsed.devices.len());
    for (orig, parse) in original.devices.iter().zip(parsed.devices.iter()) {
        assert_eq!(orig.device.name, parse.device.name);
        assert_eq!(orig.device.bus, parse.device.bus);
        assert_eq!(orig.device.path, parse.device.path);
        assert_eq!(orig.features.len(), parse.features.len());
    }
}

/// JSON with unusual values (negative temps, zero RPM).
#[test]
fn test_unusual_values() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device { name: "ext".into(), bus: "SMBus".into(), path: None },
            features: vec![FeatureInfo {
                name: "temp1".into(),
                sub_features: vec![
                    SubFeatureInfo {
                        name: "temp1_input".into(),
                        value: Some(-40.0), // Below freezing
                        unit: Some("°C".into()),
                    },
                    SubFeatureInfo {
                        name: "temp1_min".into(),
                        value: Some(0.0),
                        unit: Some("°C".into()),
                    },
                ],
            }],
        }],
    };
    let json = serde_json::to_string(&readings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["devices"][0]["features"][0]["sub_features"][0]["value"], -40.0);
}

/// Device path with special characters.
#[test]
fn test_device_path_special_chars() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device {
                name: "test".into(),
                bus: "ISA".into(),
                path: Some("/sys/class/hwmon/hwmon12/device/0-0028/temp1_input".into()),
            },
            features: vec![],
        }],
    };
    let json = serde_json::to_string(&readings).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["devices"][0]["device"]["path"]
        .as_str()
        .unwrap()
        .contains("temp1_input"));
}

// ── Type Property Tests ─────────────────────────────────────────

/// Device derives Clone, Debug, Serialize, Deserialize.
#[test]
fn test_device_traits() {
    let d = Device { name: "test".into(), bus: "ISA".into(), path: None };
    let _cloned = d.clone();
    let _debug = format!("{:?}", d);
    let _json = serde_json::to_string(&d).unwrap();
    let _parsed: Device = serde_json::from_str(&_json).unwrap();
}

/// FeatureInfo derives Clone, Debug, Serialize, Deserialize.
#[test]
fn test_feature_traits() {
    let f = FeatureInfo { name: "temp1".into(), sub_features: vec![] };
    let _cloned = f.clone();
    let _debug = format!("{:?}", f);
    let _json = serde_json::to_string(&f).unwrap();
    let _parsed: FeatureInfo = serde_json::from_str(&_json).unwrap();
}

/// SubFeatureInfo derives Clone, Debug, Serialize, Deserialize.
#[test]
fn test_subfeature_traits() {
    let s = SubFeatureInfo { name: "temp1_input".into(), value: None, unit: None };
    let _cloned = s.clone();
    let _debug = format!("{:?}", s);
    let _json = serde_json::to_string(&s).unwrap();
    let _parsed: SubFeatureInfo = serde_json::from_str(&_json).unwrap();
}

/// SensorReadings derives Clone, Debug, Serialize, Deserialize.
#[test]
fn test_sensor_readings_traits() {
    let r = SensorReadings { devices: vec![] };
    let _cloned = r.clone();
    let _debug = format!("{:?}", r);
    let _json = serde_json::to_string(&r).unwrap();
    let _parsed: SensorReadings = serde_json::from_str(&_json).unwrap();
}
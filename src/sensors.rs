use std::sync::{RwLock, Arc};

pub use self::data::*;

mod data {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ChipInfo {
        pub name: String,
        pub bus: String,
        pub path: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FeatureInfo {
        pub name: String,
        pub sub_features: Vec<SubFeatureInfo>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SubFeatureInfo {
        pub name: String,
        pub value: Option<f64>,
        pub unit: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SensorReadings {
        pub chips: Vec<ChipReadings>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ChipReadings {
        pub chip: ChipInfo,
        pub features: Vec<FeatureInfo>,
    }
}

struct SafeLMSensors(lm_sensors::LMSensors);
unsafe impl Send for SafeLMSensors {}
unsafe impl Sync for SafeLMSensors {}

pub struct SensorManager {
    sensors: Arc<RwLock<SafeLMSensors>>,
}

impl Clone for SensorManager {
    fn clone(&self) -> Self {
        Self { sensors: Arc::clone(&self.sensors) }
    }
}

unsafe impl Send for SensorManager {}
unsafe impl Sync for SensorManager {}

impl SensorManager {
    pub fn new() -> Result<Self, String> {
        let sensors = lm_sensors::Initializer::default()
            .initialize()
            .map_err(|e| format!("Failed to initialize lm-sensors: {}", e))?;
        Ok(Self { sensors: Arc::new(RwLock::new(SafeLMSensors(sensors))) })
    }

    fn with_lock<T, F: FnOnce(&lm_sensors::LMSensors) -> T>(&self, f: F) -> T {
        let s = self.sensors.read().unwrap();
        f(&s.0)
    }

    pub fn list_chips(&self) -> Vec<ChipInfo> {
        self.with_lock(|s| s.chip_iter(None).map(|c| chip_info(&c)).collect())
    }

    pub fn get_chip(&self, name: &str) -> Option<ChipInfo> {
        self.with_lock(|s| {
            s.chip_iter(None)
                .find(|c| c.name().map_or(false, |n| n.contains(name)))
                .map(|c| chip_info(&c))
        })
    }

    pub fn get_chip_features(&self, name: &str) -> Option<ChipReadings> {
        self.with_lock(|s| {
            s.chip_iter(None)
                .find(|c| c.name().map_or(false, |n| n.contains(name)))
                .map(|c| ChipReadings {
                    chip: chip_info(&c),
                    features: chip_features(&c),
                })
        })
    }

    pub fn read_all(&self) -> SensorReadings {
        self.with_lock(|s| SensorReadings {
            chips: s
                .chip_iter(None)
                .map(|c| ChipReadings {
                    chip: chip_info(&c),
                    features: chip_features(&c),
                })
                .collect(),
        })
    }
}

fn chip_info(c: &lm_sensors::ChipRef) -> ChipInfo {
    ChipInfo {
        name: c.name().unwrap_or_default().to_string(),
        bus: c.bus().to_string(),
        path: c.path().and_then(|p| p.to_str().map(str::to_string)),
    }
}

fn chip_features(c: &lm_sensors::ChipRef) -> Vec<FeatureInfo> {
    c.feature_iter()
        .filter_map(|f| {
            let name = f.name().transpose().ok().flatten().unwrap_or_default().to_string();
            let subs: Vec<_> = f
                .sub_feature_iter()
                .filter_map(|sf| {
                    match sf.value() {
                        Ok(v) => Some(SubFeatureInfo {
                            name: format!("{:?}", v.kind()),
                            value: Some(v.raw_value()),
                            unit: Some(unit_str(&v)),
                        }),
                        Err(_) => None,
                    }
                })
                .collect();
            Some(FeatureInfo { name, sub_features: subs })
        })
        .collect()
}

fn unit_str(v: &lm_sensors::Value) -> String {
    let unit = v.unit();
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

    #[test]
    fn test_readings_serde() {
        let r = SensorReadings {
            chips: vec![ChipReadings {
                chip: ChipInfo {
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
    }

    #[test]
    fn test_empty_readings() {
        let r = SensorReadings { chips: vec![] };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("chips"));
    }
}

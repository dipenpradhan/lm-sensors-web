// Integration tests for webhook configuration and sensor data

use lm_sensors_api::config::{WebhookConfig, WebhookTrigger, WebhookCondition};
use lm_sensors_api::sensors::{ChipInfo, ChipReadings, FeatureInfo, SensorReadings, SubFeatureInfo};

#[test]
fn test_webhook_config_serde() {
    let wh = WebhookConfig {
        name: "temp-alert".into(),
        url: "http://localhost:9090/alerts".into(),
        method: "POST".into(),
        content_type: "application/json".into(),
        trigger: WebhookTrigger::Temperature,
        condition: Some(WebhookCondition {
            above_celsius: Some(80.0),
            below_celsius: None,
        }),
        interval_seconds: 30,
        headers: std::collections::HashMap::new(),
    };
    let j = serde_json::to_string(&wh).unwrap();
    assert!(j.contains("temp-alert"));
    assert!(j.contains("temperature"));
    assert!(j.contains("80"));
}

#[test]
fn test_sensor_readings_temps() {
    let readings = SensorReadings {
        chips: vec![ChipReadings {
            chip: ChipInfo {
                name: "coretemp".into(),
                bus: "ISA".into(),
                path: None,
            },
            features: vec![
                FeatureInfo {
                    name: "temp1_input".into(),
                    sub_features: vec![SubFeatureInfo {
                        name: "temp1_input".into(),
                        value: Some(60.0),
                        unit: Some("°C".into()),
                    }],
                },
                FeatureInfo {
                    name: "temp2_input".into(),
                    sub_features: vec![SubFeatureInfo {
                        name: "temp2_input".into(),
                        value: Some(85.0),
                        unit: Some("°C".into()),
                    }],
                },
            ],
        }],
    };

    let temps: Vec<f64> = readings.chips.iter()
        .flat_map(|c| c.features.iter())
        .flat_map(|f| f.sub_features.iter())
        .filter_map(|s| s.value)
        .collect();

    let avg = temps.iter().sum::<f64>() / temps.len() as f64;
    assert!((avg - 72.5).abs() < 0.01);
}

#[test]
fn test_webhook_config_roundtrip() {
    let original = WebhookConfig {
        name: "test".into(),
        url: "http://example.com/hook".into(),
        method: "POST".into(),
        content_type: "application/json".into(),
        trigger: WebhookTrigger::OnChange,
        condition: None,
        interval_seconds: 60,
        headers: {
            let mut m = std::collections::HashMap::new();
            m.insert("X-API-Key".into(), "secret".into());
            m
        },
    };
    let j = serde_json::to_string(&original).unwrap();
    let parsed: WebhookConfig = serde_json::from_str(&j).unwrap();
    assert_eq!(original.name, parsed.name);
    assert_eq!(original.url, parsed.url);
    assert_eq!(original.interval_seconds, parsed.interval_seconds);
}

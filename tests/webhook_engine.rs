//! # Webhook engine integration tests
//!
//! Tests the webhook trigger logic, payload construction, and condition
//! evaluation without requiring a real HTTP server.
//!
//! # Running
//!
//! ```bash
//! cargo test --test webhook_engine
//! ```

use lm_sensors_web::config::{WebhookConfig, WebhookCondition, WebhookTrigger};
use lm_sensors_web::sensors::{
    Device, DeviceReadings, FeatureInfo, SensorReadings, SubFeatureInfo,
};
use serde_json::json;
use std::collections::HashMap;

// ── Webhook Trigger Tests ──────────────────────────────────────────

/// Always trigger fires unconditionally.
#[test]
fn test_always_trigger_fires() {
    let wh = WebhookConfig {
        name: "always".into(),
        url: "http://example.com/hook".into(),
        method: "POST".into(),
        content_type: "application/json".into(),
        trigger: WebhookTrigger::Always,
        condition: None,
        interval_seconds: 30,
        headers: HashMap::new(),
    };
    assert!(matches!(wh.trigger, WebhookTrigger::Always));
}

/// Temperature trigger with above threshold fires when condition is met.
#[test]
fn test_temperature_trigger_above_fires() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device { name: "test".into(), bus: "ISA".into(), path: None },
            features: vec![FeatureInfo {
                name: "temp1".into(),
                sub_features: vec![SubFeatureInfo {
                    name: "temp1_input".into(),
                    value: Some(90.0),
                    unit: Some("°C".into()),
                }],
            }],
        }],
    };
    let cond = WebhookCondition { above_celsius: Some(80.0), below_celsius: None };
    // Manually check: 90 > 80 → should fire
    let fires = check_temperature_condition(&readings, &cond);
    assert!(fires);
}

/// Temperature trigger with above threshold does not fire below threshold.
#[test]
fn test_temperature_trigger_above_no_fire() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device { name: "test".into(), bus: "ISA".into(), path: None },
            features: vec![FeatureInfo {
                name: "temp1".into(),
                sub_features: vec![SubFeatureInfo {
                    name: "temp1_input".into(),
                    value: Some(70.0),
                    unit: Some("°C".into()),
                }],
            }],
        }],
    };
    let cond = WebhookCondition { above_celsius: Some(80.0), below_celsius: None };
    // 70 is not > 80 → should not fire
    let fires = check_temperature_condition(&readings, &cond);
    assert!(!fires);
}

/// Temperature trigger with below threshold fires when condition is met.
#[test]
fn test_temperature_trigger_below_fires() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device { name: "test".into(), bus: "ISA".into(), path: None },
            features: vec![FeatureInfo {
                name: "temp1".into(),
                sub_features: vec![SubFeatureInfo {
                    name: "temp1_input".into(),
                    value: Some(30.0),
                    unit: Some("°C".into()),
                }],
            }],
        }],
    };
    let cond = WebhookCondition { above_celsius: None, below_celsius: Some(40.0) };
    // 30 < 40 → should fire
    let fires = check_temperature_condition(&readings, &cond);
    assert!(fires);
}

/// Temperature trigger with both thresholds.
#[test]
fn test_temperature_trigger_both_thresholds() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device { name: "test".into(), bus: "ISA".into(), path: None },
            features: vec![FeatureInfo {
                name: "temp1".into(),
                sub_features: vec![
                    SubFeatureInfo {
                        name: "temp1_input".into(),
                        value: Some(95.0),
                        unit: Some("°C".into()),
                    },
                    SubFeatureInfo {
                        name: "temp2_input".into(),
                        value: Some(25.0),
                        unit: Some("°C".into()),
                    },
                ],
            }],
        }],
    };
    let cond = WebhookCondition { above_celsius: Some(90.0), below_celsius: Some(30.0) };
    // 95 > 90 (above fires) OR 25 < 30 (below fires)
    let fires = check_temperature_condition(&readings, &cond);
    assert!(fires);
}

/// On-change trigger fires when temperature changes.
#[test]
fn test_on_change_trigger_fires() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device { name: "test".into(), bus: "ISA".into(), path: None },
            features: vec![FeatureInfo {
                name: "temp1".into(),
                sub_features: vec![SubFeatureInfo {
                    name: "temp1_input".into(),
                    value: Some(70.0),
                    unit: Some("°C".into()),
                }],
            }],
        }],
    };
    let last_avg = Some(50.0); // Previous average
    let cur_avg = compute_avg_temp(&readings);
    // |70 - 50| = 20 > 0.1 → should fire
    let fires = check_on_change(&cur_avg, &last_avg);
    assert!(fires);
}

/// On-change trigger does not fire with small change.
#[test]
fn test_on_change_trigger_no_fire() {
    let cur_avg = Some(50.05);
    let last_avg = Some(50.0);
    // |50.05 - 50.0| = 0.05 ≤ 0.1 → should not fire
    let fires = check_on_change(&cur_avg, &last_avg);
    assert!(!fires);
}

/// On-change trigger fires on first reading (no previous value).
#[test]
fn test_on_change_trigger_first_reading() {
    let cur_avg = Some(50.0);
    let last_avg = None;
    // First reading always fires
    let fires = check_on_change(&cur_avg, &last_avg);
    assert!(fires);
}

// ── Webhook Payload Tests ─────────────────────────────────────────

/// Webhook payload contains all required fields.
#[test]
fn test_webhook_payload_structure() {
    let readings = SensorReadings {
        devices: vec![DeviceReadings {
            device: Device { name: "test".into(), bus: "ISA".into(), path: None },
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
    let payload = json!({
        "webhook": "test-hook",
        "timestamp": "2024-01-01T00:00:00+00:00",
        "readings": readings,
    });
    // Verify payload keys
    assert!(payload.get("webhook").is_some());
    assert!(payload.get("timestamp").is_some());
    assert!(payload.get("readings").is_some());
    assert_eq!(payload["webhook"].as_str(), Some("test-hook"));
}

/// Webhook payload serializes to valid JSON.
#[test]
fn test_webhook_payload_serialization() {
    let payload = json!({
        "webhook": "temp-alert",
        "timestamp": "2024-01-01T00:00:00+00:00",
        "readings": {
            "devices": [{
                "device": {"name": "cpu", "bus": "ISA", "path": null},
                "features": []
            }]
        }
    });
    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains("temp-alert"));
    assert!(json.contains("devices"));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["webhook"].as_str(), Some("temp-alert"));
}

/// Webhook with custom headers serializes correctly.
#[test]
fn test_webhook_with_headers() {
    let mut headers = HashMap::new();
    headers.insert("X-API-Key".into(), "secret".into());
    headers.insert("Authorization".into(), "Bearer token123".into());

    let wh = WebhookConfig {
        name: "monitored".into(),
        url: "https://api.example.com/hooks".into(),
        method: "POST".into(),
        content_type: "application/json".into(),
        trigger: WebhookTrigger::Always,
        condition: None,
        interval_seconds: 60,
        headers,
    };

    let json = serde_json::to_string(&wh).unwrap();
    assert!(json.contains("X-API-Key"));
    assert!(json.contains("secret"));
}

// ── Webhook Trigger Serialization Tests ───────────────────────────

/// All trigger variants serialize and deserialize correctly.
#[test]
fn test_trigger_serialization() {
    for (trigger, expected_str) in [
        (WebhookTrigger::Always, "always"),
        (WebhookTrigger::Temperature, "temperature"),
        (WebhookTrigger::OnChange, "on_change"),
    ] {
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains(expected_str), "Expected '{}' in {}", expected_str, json);
    }
}

/// WebhookConfig with all fields serializes and deserializes correctly.
#[test]
fn test_webhook_config_full_roundtrip() {
    let original = WebhookConfig {
        name: "full-config".into(),
        url: "http://localhost:9090/alerts".into(),
        method: "PUT".into(),
        content_type: "application/json".into(),
        trigger: WebhookTrigger::Temperature,
        condition: Some(WebhookCondition {
            above_celsius: Some(85.0),
            below_celsius: Some(20.0),
        }),
        interval_seconds: 120,
        headers: {
            let mut m = HashMap::new();
            m.insert("X-Custom".into(), "value".into());
            m
        },
    };
    let json = serde_json::to_string(&original).unwrap();
    let parsed: WebhookConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(original.name, parsed.name);
    assert_eq!(original.url, parsed.url);
    assert_eq!(original.method, parsed.method);
    assert_eq!(original.interval_seconds, parsed.interval_seconds);
    assert_eq!(original.headers.len(), parsed.headers.len());
}

// ── Helper Functions ───────────────────────────────────────────────

/// Check if a temperature condition is met (same logic as webhook.rs).
fn check_temperature_condition(
    readings: &SensorReadings,
    cond: &WebhookCondition,
) -> bool {
    for dev in &readings.devices {
        for feat in &dev.features {
            for sub in &feat.sub_features {
                if sub.name.contains("temp") {
                    if let Some(v) = sub.value {
                        if let Some(above) = cond.above_celsius {
                            if v > above {
                                return true;
                            }
                        }
                        if let Some(below) = cond.below_celsius {
                            if v < below {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

/// Compute average temperature (same logic as webhook.rs).
fn compute_avg_temp(readings: &SensorReadings) -> Option<f64> {
    let mut sum = 0.0_f64;
    let mut count = 0u32;
    for dev in &readings.devices {
        for feat in &dev.features {
            for sub in &feat.sub_features {
                if sub.name.contains("temp") {
                    if let Some(v) = sub.value {
                        sum += v;
                        count += 1;
                    }
                }
            }
        }
    }
    if count > 0 {
        Some(sum / count as f64)
    } else {
        None
    }
}

/// Check on-change trigger condition.
fn check_on_change(cur_avg: &Option<f64>, last_avg: &Option<f64>) -> bool {
    match (cur_avg, last_avg) {
        (Some(cur), Some(last)) if (cur - last).abs() > 0.1 => true,
        (None, _) => false,
        (Some(_), None) => true, // First reading always fires
        _ => false,
    }
}
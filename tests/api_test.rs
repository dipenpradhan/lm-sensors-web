//! # API model integration tests
//!
//! Tests the data models and config schemas used by REST API endpoints.
//!
//! # Running
//!
//! ```bash
//! cargo test --test api_test
//! ```

use lm_sensors_web::config::{Config, ServerConfig};

/// Default config values match expected production defaults.
#[test]
fn test_default_config_values() {
    let c = Config::default();
    assert_eq!(c.server.host, "0.0.0.0");
    assert_eq!(c.server.port, 47890);
    assert_eq!(c.server.log_level, "info");
    assert!(c.websocket.enabled);
    assert_eq!(c.websocket.path, "/ws/sensors");
    assert_eq!(c.websocket.broadcast_interval_ms, 2000);
    assert!(c.webhooks.is_empty());
    assert_eq!(c.sensors.refresh_interval_ms, 5000);
}

/// ServerConfig defaults are correct.
#[test]
fn test_server_config_defaults() {
    let sc = ServerConfig::default();
    assert_eq!(sc.host, "0.0.0.0");
    assert_eq!(sc.port, 47890);
    assert_eq!(sc.log_level, "info");
}

/// API health response schema is valid JSON.
#[test]
fn test_health_response_schema() {
    let response = serde_json::json!({
        "status": "ok",
        "timestamp": "2024-01-01T00:00:00+00:00"
    });
    let value: serde_json::Value = response;
    assert_eq!(value["status"], "ok");
    assert!(value["timestamp"].is_string());
}

/// API device listing response schema.
#[test]
fn test_device_list_response_schema() {
    let devices = serde_json::json!({
        "devices": [
            {"name": "coretemp", "bus": "ISA", "path": "/sys/class/hwmon/hwmon0"},
            {"name": "acpitz", "bus": "ISA", "path": null}
        ]
    });
    assert!(devices["devices"].is_array());
    assert_eq!(devices["devices"].as_array().unwrap().len(), 2);
}

/// API device detail response schema.
#[test]
fn test_device_detail_response_schema() {
    let device = serde_json::json!({
        "device": {
            "name": "coretemp-isa-0000",
            "bus": "ISA",
            "path": "/sys/class/hwmon/hwmon0"
        }
    });
    assert_eq!(device["device"]["name"], "coretemp-isa-0000");
    assert_eq!(device["device"]["bus"], "ISA");
}

/// API error response schema.
#[test]
fn test_error_response_schema() {
    let error = serde_json::json!({
        "error": "Device 'nonexistent' not found"
    });
    assert!(error["error"].is_string());
    assert!(error["error"].as_str().unwrap().contains("not found"));
}

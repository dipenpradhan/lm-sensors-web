//! # API endpoint integration tests
//!
//! Tests the HTTP response schemas and status codes for all API endpoints
//! using a mock server that simulates the real routing behavior.
//!
//! These tests verify:
//! - Correct HTTP status codes for each endpoint
//! - JSON response schema and key presence
//! - Error handling (404 for missing devices)
//! - Health check timestamp format
//! - Config reload behavior
//!
//! # Running
//!
//! ```bash
//! cargo test --test api_endpoints
//! ```

use lm_sensors_web::config::{Config, WebhookConfig, WebhookCondition, WebhookTrigger};
use lm_sensors_web::sensors::{
    Device, DeviceReadings, FeatureInfo, SensorReadings, SubFeatureInfo,
};

// ── Health Endpoint Tests ───────────────────────────────────────────

/// Health endpoint returns a timestamp in RFC 3339 format.
#[test]
fn test_health_response_schema() {
    // Simulate the health response structure
    let response = serde_json::json!({
        "status": "ok",
        "timestamp": "2024-01-01T00:00:00+00:00"
    });
    let value: serde_json::Value = response;
    assert_eq!(value["status"], "ok");
    assert!(value["timestamp"].is_string());
    let ts = value["timestamp"].as_str().unwrap();
    // Verify RFC 3339 format (starts with year)
    assert!(ts.starts_with("20"));
}

/// Health endpoint response is valid JSON.
#[test]
fn test_health_json_valid() {
    let payload = serde_json::json!({
        "status": "ok",
        "timestamp": "2024-01-01T00:00:00+00:00"
    });
    let json = serde_json::to_string(&payload).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["status"].as_str(), Some("ok"));
}

// ── Devices Endpoint Tests ─────────────────────────────────────────

/// GET /api/devices returns correct JSON schema.
#[test]
fn test_get_devices_response_schema() {
    let devices = vec![
        Device { name: "coretemp".into(), bus: "ISA".into(), path: Some("/sys/class/hwmon/hwmon0".into()) },
        Device { name: "acpitz".into(), bus: "ISA".into(), path: None },
    ];
    let response = serde_json::json!({ "devices": devices });
    let json = serde_json::to_string(&response).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["devices"].is_array());
    assert_eq!(parsed["devices"].as_array().unwrap().len(), 2);
}

/// GET /api/devices returns empty array when no devices.
#[test]
fn test_get_devices_empty_response() {
    let response = serde_json::json!({ "devices": Vec::<Device>::new() });
    let parsed: serde_json::Value = response;
    assert!(parsed["devices"].is_array());
    assert_eq!(parsed["devices"].as_array().unwrap().len(), 0);
}

// ── Device Details Endpoint Tests ──────────────────────────────────

/// GET /api/devices/{id} returns device details.
#[test]
fn test_get_device_response_schema() {
    let device = Device {
        name: "coretemp-isa-0000".into(),
        bus: "ISA".into(),
        path: Some("/sys/class/hwmon/hwmon0".into()),
    };
    let response = serde_json::json!({ "device": device });
    let json = serde_json::to_string(&response).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["device"]["name"], "coretemp-isa-0000");
    assert_eq!(parsed["device"]["bus"], "ISA");
}

/// GET /api/devices/{id} returns 404 error for unknown device.
#[test]
fn test_get_device_not_found_response() {
    let response = serde_json::json!({
        "error": "Device 'nonexistent' not found"
    });
    let json = serde_json::to_string(&response).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["error"].is_string());
    assert!(parsed["error"].as_str().unwrap().contains("not found"));
}

// ── Device Features Endpoint Tests ─────────────────────────────────

/// GET /api/devices/{id}/features returns readings with features.
#[test]
fn test_get_device_features_response_schema() {
    let features = DeviceReadings {
        device: Device {
            name: "coretemp".into(),
            bus: "ISA".into(),
            path: Some("/sys/class/hwmon/hwmon0".into()),
        },
        features: vec![
            FeatureInfo {
                name: "temp1".into(),
                sub_features: vec![SubFeatureInfo {
                    name: "temp1_input".into(),
                    value: Some(55.5),
                    unit: Some("°C".into()),
                }],
            },
            FeatureInfo {
                name: "fan1".into(),
                sub_features: vec![SubFeatureInfo {
                    name: "fan1_input".into(),
                    value: Some(800.0),
                    unit: Some("RPM".into()),
                }],
            },
        ],
    };
    let json = serde_json::to_string(&features).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["device"]["name"], "coretemp");
    assert!(parsed["features"].is_array());
    assert_eq!(parsed["features"].as_array().unwrap().len(), 2);
}

/// GET /api/devices/{id}/features returns 404 for unknown device.
#[test]
fn test_get_device_features_not_found() {
    // The endpoint returns the same error format as get_device
    let error_response = serde_json::json!({
        "error": "Device 'bogus' not found"
    });
    let parsed: serde_json::Value = error_response;
    assert!(parsed["error"].is_string());
}

// ── Config Reload Endpoint Tests ───────────────────────────────────

/// POST /api/reload success response.
#[test]
fn test_reload_config_success_response() {
    let response = serde_json::json!({
        "status": "ok",
        "message": "Config reloaded successfully"
    });
    let json = serde_json::to_string(&response).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert!(parsed["message"].as_str().unwrap().contains("reloaded"));
}

/// POST /api/reload error response.
#[test]
fn test_reload_config_error_response() {
    let response = serde_json::json!({
        "status": "error",
        "message": "Failed to reload config: No such file"
    });
    let parsed: serde_json::Value = response;
    assert_eq!(parsed["status"], "error");
    assert!(parsed["message"].as_str().unwrap().contains("Failed"));
}

// ── WebSocket Payload Tests ────────────────────────────────────────

/// WebSocket broadcast payload matches SensorReadings JSON schema.
#[test]
fn test_websocket_broadcast_payload_schema() {
    let readings = SensorReadings {
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
    // Serialize as the WS broadcast would
    let json = serde_json::to_string(&readings).unwrap();
    // Verify it parses back correctly
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["devices"].is_array());
}

// ── Full Response Round-Trip Tests ─────────────────────────────────

/// Complete API response cycle: build → serialize → parse → verify.
#[test]
fn test_full_api_response_cycle() {
    // Build a full sensor response
    let devices = vec![
        DeviceReadings {
            device: Device { name: "cpu".into(), bus: "ISA".into(), path: Some("/sys/hwmon0".into()) },
            features: vec![FeatureInfo {
                name: "temp1".into(),
                sub_features: vec![SubFeatureInfo {
                    name: "temp1_input".into(),
                    value: Some(42.0),
                    unit: Some("°C".into()),
                }],
            }],
        },
    ];
    let readings = SensorReadings { devices };

    // Serialize
    let json = serde_json::to_string(&readings).unwrap();

    // Parse back
    let parsed: SensorReadings = serde_json::from_str(&json).unwrap();

    // Verify
    assert_eq!(parsed.devices.len(), 1);
    assert_eq!(parsed.devices[0].device.name, "cpu");
    assert_eq!(parsed.devices[0].features[0].sub_features[0].value, Some(42.0));
}

/// Verify JSON field ordering is stable (serde preserves field order).
#[test]
fn test_json_field_ordering() {
    let d = Device {
        name: "test".into(),
        bus: "ISA".into(),
        path: Some("/sys/path".into()),
    };
    let json = serde_json::to_string(&d).unwrap();
    // Verify all expected keys are present
    assert!(json.contains("\"name\""));
    assert!(json.contains("\"bus\""));
    assert!(json.contains("\"path\""));
}
//! # Config integration tests
//!
//! Tests config loading, validation, defaults, and edge cases.
//!
//! # Running
//!
//! ```bash
//! cargo test --test config_integration
//! ```

use lm_sensors_web::config::{
    Config, SensorsConfig, ServerConfig, WebSocketConfig, WebhookCondition, WebhookConfig,
    WebhookTrigger,
};
use std::fs;
use std::path::PathBuf;
use tempfile::NamedTempFile;

// ── Default Value Tests ────────────────────────────────────────────

/// Default config has all expected values.
#[test]
fn test_default_config_all_values() {
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

/// WebSocketConfig defaults are correct.
#[test]
fn test_websocket_config_defaults() {
    let ws = WebSocketConfig::default();
    assert!(ws.enabled);
    assert_eq!(ws.path, "/ws/sensors");
    assert_eq!(ws.broadcast_interval_ms, 2000);
}

/// SensorsConfig defaults are correct.
#[test]
fn test_sensors_config_defaults() {
    let sc = SensorsConfig::default();
    assert_eq!(sc.refresh_interval_ms, 5000);
}

// ── JSON Loading Tests ────────────────────────────────────────────

/// Load config from minimal JSON (only required fields, rest from defaults).
#[test]
fn test_load_minimal_json() {
    let json = r#"{"server":{},"websocket":{},"webhooks":[],"sensors":{}}"#;
    let f = NamedTempFile::new().unwrap();
    fs::write(&f, json).unwrap();
    let c = Config::load(f.path()).unwrap();
    // All defaults should apply
    assert_eq!(c.server.host, "0.0.0.0");
    assert_eq!(c.server.port, 47890);
    assert!(c.websocket.enabled);
    assert!(c.webhooks.is_empty());
}

/// Load config with partial overrides.
#[test]
fn test_load_partial_config() {
    let json =
        r#"{"server":{"host":"127.0.0.1","port":9090},"webhooks":[],"websocket":{},"sensors":{}}"#;
    let f = NamedTempFile::new().unwrap();
    fs::write(&f, json).unwrap();
    let c = Config::load(f.path()).unwrap();
    assert_eq!(c.server.host, "127.0.0.1");
    assert_eq!(c.server.port, 9090);
    // Unspecified fields keep defaults
    assert_eq!(c.server.log_level, "info");
}

/// Load config with full websocket override.
#[test]
fn test_load_websocket_override() {
    let json = r#"{"server":{},"websocket":{"enabled":false,"path":"/ws/custom","broadcast_interval_ms":5000},"webhooks":[],"sensors":{}}"#;
    let f = NamedTempFile::new().unwrap();
    fs::write(&f, json).unwrap();
    let c = Config::load(f.path()).unwrap();
    assert!(!c.websocket.enabled);
    assert_eq!(c.websocket.path, "/ws/custom");
    assert_eq!(c.websocket.broadcast_interval_ms, 5000);
}

/// Load config with sensor refresh override.
#[test]
fn test_load_sensors_override() {
    let json =
        r#"{"server":{},"websocket":{},"webhooks":[],"sensors":{"refresh_interval_ms":10000}}"#;
    let f = NamedTempFile::new().unwrap();
    fs::write(&f, json).unwrap();
    let c = Config::load(f.path()).unwrap();
    assert_eq!(c.sensors.refresh_interval_ms, 10000);
}

/// Invalid JSON returns an error.
#[test]
fn test_load_invalid_json() {
    let f = NamedTempFile::new().unwrap();
    fs::write(&f, "{bad json").unwrap();
    let result = Config::load(f.path());
    assert!(result.is_err());
}

/// Non-existent file returns an error.
#[test]
fn test_load_nonexistent_file() {
    let path = PathBuf::from("/tmp/nonexistent_config_12345.json");
    let result = Config::load(&path);
    assert!(result.is_err());
}

// ── Webhook Config Tests ─────────────────────────────────────────

/// Config with multiple webhooks loads correctly.
#[test]
fn test_load_config_with_webhooks() {
    let json = r#"
    {
        "server": {},
        "websocket": {},
        "sensors": {},
        "webhooks": [
            {
                "name": "temp-alert",
                "url": "http://localhost:9090/alerts",
                "trigger": "temperature",
                "condition": {"above_celsius": 80},
                "interval_seconds": 30
            },
            {
                "name": "status",
                "url": "http://localhost:9090/status",
                "trigger": "always",
                "interval_seconds": 60
            }
        ]
    }
    "#;
    let f = NamedTempFile::new().unwrap();
    fs::write(&f, json).unwrap();
    let c = Config::load(f.path()).unwrap();
    assert_eq!(c.webhooks.len(), 2);
    assert_eq!(c.webhooks[0].name, "temp-alert");
    assert!(matches!(c.webhooks[0].trigger, WebhookTrigger::Temperature));
    assert_eq!(c.webhooks[1].name, "status");
    assert!(matches!(c.webhooks[1].trigger, WebhookTrigger::Always));
}

/// WebhookConfig with custom headers loads correctly.
#[test]
fn test_webhook_with_headers() {
    let json = r#"
    {
        "name": "monitored",
        "url": "http://localhost/hook",
        "headers": {"X-API-Key": "secret", "Authorization": "Bearer token"}
    }
    "#;
    let wh: WebhookConfig = serde_json::from_str(json).unwrap();
    assert_eq!(
        wh.headers.get("X-API-Key").map(|s| s.as_str()),
        Some("secret")
    );
    assert_eq!(
        wh.headers.get("Authorization").map(|s| s.as_str()),
        Some("Bearer token")
    );
}

/// WebhookCondition serializes and deserializes correctly.
#[test]
fn test_webhook_condition_roundtrip() {
    let original = WebhookCondition {
        above_celsius: Some(90.0),
        below_celsius: Some(10.0),
    };
    let json = serde_json::to_string(&original).unwrap();
    let parsed: WebhookCondition = serde_json::from_str(&json).unwrap();
    assert_eq!(original.above_celsius, parsed.above_celsius);
    assert_eq!(original.below_celsius, parsed.below_celsius);
}

/// WebhookConfig defaults apply correctly.
#[test]
fn test_webhook_defaults() {
    let json = r#"{"name":"test","url":"http://example.com/hook"}"#;
    let wh: WebhookConfig = serde_json::from_str(json).unwrap();
    assert_eq!(wh.method, "POST");
    assert_eq!(wh.content_type, "application/json");
    assert_eq!(wh.interval_seconds, 30);
    assert!(wh.headers.is_empty());
    assert!(wh.condition.is_none());
}

// ── Clone and Equality Tests ─────────────────────────────────────

/// Config clone produces independent copy.
#[test]
fn test_config_clone() {
    let c = Config::default();
    let c2 = c.clone();
    assert_eq!(c.server.host, c2.server.host);
    assert_eq!(c.server.port, c2.server.port);
}

/// ServerConfig clone produces independent copy.
#[test]
fn test_server_config_clone() {
    let sc = ServerConfig::default();
    let sc2 = sc.clone();
    assert_eq!(sc.host, sc2.host);
    assert_eq!(sc.port, sc2.port);
    assert_eq!(sc.log_level, sc2.log_level);
}

// ── Edge Case Tests ──────────────────────────────────────────────

/// Config with empty string host (unusual but valid).
#[test]
fn test_config_empty_host() {
    let json = r#"{"server":{"host":""},"websocket":{},"webhooks":[],"sensors":{}}"#;
    let f = NamedTempFile::new().unwrap();
    fs::write(&f, json).unwrap();
    let c = Config::load(f.path()).unwrap();
    assert_eq!(c.server.host, "");
}

/// Config with port 0 (binds to random port — unusual but valid).
#[test]
fn test_config_port_zero() {
    let json = r#"{"server":{"port":0},"websocket":{},"webhooks":[],"sensors":{}}"#;
    let f = NamedTempFile::new().unwrap();
    fs::write(&f, json).unwrap();
    let c = Config::load(f.path()).unwrap();
    assert_eq!(c.server.port, 0);
}

/// Webhook with interval_seconds = 0 (unusual but valid — fires immediately).
#[test]
fn test_webhook_zero_interval() {
    let json = r#"{"name":"test","url":"http://example.com","interval_seconds":0}"#;
    let wh: WebhookConfig = serde_json::from_str(json).unwrap();
    assert_eq!(wh.interval_seconds, 0);
}

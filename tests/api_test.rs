// Integration tests for REST API data models and CLI

use lm_sensors_api::config::{Config, ServerConfig};

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

#[test]
fn test_server_config_defaults() {
    let sc = ServerConfig::default();
    assert_eq!(sc.host, "0.0.0.0");
    assert_eq!(sc.port, 47890);
    assert_eq!(sc.log_level, "info");
}

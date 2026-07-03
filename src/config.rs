use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_server")]
    pub server: ServerConfig,
    #[serde(default = "default_websocket")]
    pub websocket: WebSocketConfig,
    #[serde(default = "default_webhooks")]
    pub webhooks: Vec<WebhookConfig>,
    #[serde(default = "default_sensors")]
    pub sensors: SensorsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    #[serde(default = "default_ws_enabled")]
    pub enabled: bool,
    #[serde(default = "default_ws_path")]
    pub path: String,
    #[serde(default = "default_broadcast_interval_ms")]
    pub broadcast_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub name: String,
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default)]
    pub trigger: WebhookTrigger,
    #[serde(default)]
    pub condition: Option<WebhookCondition>,
    #[serde(default = "default_interval_seconds")]
    pub interval_seconds: u64,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WebhookTrigger {
    #[default]
    Always,
    Temperature,
    OnChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookCondition {
    #[serde(default)]
    pub above_celsius: Option<f64>,
    #[serde(default)]
    pub below_celsius: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorsConfig {
    #[serde(default = "default_refresh_interval_ms")]
    pub refresh_interval_ms: u64,
}

// — defaults —
fn default_server() -> ServerConfig  { ServerConfig::default() }
fn default_websocket() -> WebSocketConfig { WebSocketConfig::default() }
fn default_webhooks() -> Vec<WebhookConfig> { Vec::new() }
fn default_sensors() -> SensorsConfig { SensorsConfig::default() }

fn default_host() -> String { "0.0.0.0".into() }
fn default_port() -> u16 { 47890 }
fn default_log_level() -> String { "info".into() }

fn default_ws_enabled() -> bool { true }
fn default_ws_path() -> String { "/ws/sensors".into() }
fn default_broadcast_interval_ms() -> u64 { 2000 }

fn default_method() -> String { "POST".into() }
fn default_content_type() -> String { "application/json".into() }
fn default_interval_seconds() -> u64 { 30 }
fn default_refresh_interval_ms() -> u64 { 5000 }

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            websocket: WebSocketConfig::default(),
            webhooks: Vec::new(),
            sensors: SensorsConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            log_level: default_log_level(),
        }
    }
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: default_ws_enabled(),
            path: default_ws_path(),
            broadcast_interval_ms: default_broadcast_interval_ms(),
        }
    }
}

impl Default for SensorsConfig {
    fn default() -> Self {
        Self {
            refresh_interval_ms: default_refresh_interval_ms(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
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
    fn test_load_json() {
        let json = r#"{"server":{"host":"127.0.0.1","port":9090},"webhooks":[],"websocket":{},"sensors":{}}"#;
        let f = NamedTempFile::new().unwrap();
        std::fs::write(&f, json).unwrap();
        let c = Config::load(f.path()).unwrap();
        assert_eq!(c.server.host, "127.0.0.1");
        assert_eq!(c.server.port, 9090);
    }

    #[test]
    fn test_load_bad_json() {
        let f = NamedTempFile::new().unwrap();
        std::fs::write(&f, "{bad").unwrap();
        assert!(Config::load(f.path()).is_err());
    }

    #[test]
    fn test_serde_roundtrip() {
        let c = Config::default();
        let j = serde_json::to_string(&c).unwrap();
        let c2: Config = serde_json::from_str(&j).unwrap();
        assert_eq!(c.server.host, c2.server.host);
        assert_eq!(c.server.port, c2.server.port);
    }
}

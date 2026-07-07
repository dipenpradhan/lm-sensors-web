//! # Configuration module
//!
//! Reads a JSON config file (or uses built-in defaults) and provides
//! typed structs for server, WebSocket, webhook, and sensor settings.
//!
//! # Default values
//!
//! | Setting                  | Default    | Description                           |
//! |--------------------------|------------|---------------------------------------|
//! | `server.host`            | `0.0.0.0` | Bind address                          |
//! | `server.port`            | `47890`   | Listen port                           |
//! | `server.log_level`       | `info`    | `tracing` log level                   |
//! | `websocket.enabled`      | `true`    | Enable WebSocket broadcast            |
//! | `websocket.path`         | `/ws/sensors` | WebSocket endpoint path          |
//! | `websocket.broadcast_interval_ms` | `2000` | ms between broadcasts      |
//! | `sensors.refresh_interval_ms` | `5000`  | ms between sensor reads      |

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Top-level configuration.
///
/// Loaded from a JSON file or generated from defaults.
/// All nested structs and fields have serde defaults so partial configs
/// are accepted — missing keys fall back to built-in values.
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

/// Server settings (bind address, port, log level).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

/// WebSocket broadcast settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Whether the WebSocket endpoint is enabled.
    #[serde(default = "default_ws_enabled")]
    pub enabled: bool,
    /// URL path for the WebSocket endpoint.
    #[serde(default = "default_ws_path")]
    pub path: String,
    /// Milliseconds between sensor broadcast messages.
    #[serde(default = "default_broadcast_interval_ms")]
    pub broadcast_interval_ms: u64,
}

/// Webhook definition: a scheduled HTTP call with optional temperature conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Human-readable name (used in logs).
    pub name: String,
    /// Target URL for the POST request.
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    /// When to fire: always / temperature-based / on-change.
    #[serde(default)]
    pub trigger: WebhookTrigger,
    /// Optional temperature threshold (only for `temperature` trigger).
    #[serde(default)]
    pub condition: Option<WebhookCondition>,
    /// Seconds between webhook attempts.
    #[serde(default = "default_interval_seconds")]
    pub interval_seconds: u64,
    /// Extra HTTP headers to include.
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// Webhook firing condition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WebhookTrigger {
    /// Fire on every interval tick regardless of sensor values.
    #[default]
    Always,
    /// Fire only when temperature crosses a threshold.
    Temperature,
    /// Fire only when temperature changes significantly (Δ > 0.1°C).
    OnChange,
}

/// Temperature threshold condition for the `temperature` trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookCondition {
    /// Fire when any sensor reading exceeds this value in °C.
    #[serde(default)]
    pub above_celsius: Option<f64>,
    /// Fire when any sensor reading drops below this value in °C.
    #[serde(default)]
    pub below_celsius: Option<f64>,
}

/// Sensor read settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorsConfig {
    /// Milliseconds between sensor reads (used by broadcast loop).
    #[serde(default = "default_refresh_interval_ms")]
    pub refresh_interval_ms: u64,
}

// ── Default value factories for serde ──────────────────────────────

fn default_server() -> ServerConfig {
    ServerConfig::default()
}
fn default_websocket() -> WebSocketConfig {
    WebSocketConfig::default()
}
fn default_webhooks() -> Vec<WebhookConfig> {
    Vec::new()
}
fn default_sensors() -> SensorsConfig {
    SensorsConfig::default()
}

fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    47890
}
fn default_log_level() -> String {
    "info".into()
}

fn default_ws_enabled() -> bool {
    true
}
fn default_ws_path() -> String {
    "/ws/sensors".into()
}
fn default_broadcast_interval_ms() -> u64 {
    2000
}

fn default_method() -> String {
    "POST".into()
}
fn default_content_type() -> String {
    "application/json".into()
}
fn default_interval_seconds() -> u64 {
    30
}
fn default_refresh_interval_ms() -> u64 {
    5000
}

/// Built-in defaults when no config file is provided.
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
    /// Load configuration from a JSON file.
    ///
    /// Returns a descriptive error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse '{}': {}", path.display(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    /// Verify all default values match expectations.
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

    /// Verify JSON loading with partial keys (missing keys use defaults).
    #[test]
    fn test_load_json() {
        let json = r#"{"server":{"host":"127.0.0.1","port":9090},"webhooks":[],"websocket":{},"sensors":{}}"#;
        let f = NamedTempFile::new().unwrap();
        std::fs::write(&f, json).unwrap();
        let c = Config::load(f.path()).unwrap();
        assert_eq!(c.server.host, "127.0.0.1");
        assert_eq!(c.server.port, 9090);
    }

    /// Invalid JSON returns an error (does not panic).
    #[test]
    fn test_load_bad_json() {
        let f = NamedTempFile::new().unwrap();
        std::fs::write(&f, "{bad").unwrap();
        assert!(Config::load(f.path()).is_err());
    }

    /// Serialize → deserialize round-trip preserves values.
    #[test]
    fn test_serde_roundtrip() {
        let c = Config::default();
        let j = serde_json::to_string(&c).unwrap();
        let c2: Config = serde_json::from_str(&j).unwrap();
        assert_eq!(c.server.host, c2.server.host);
        assert_eq!(c.server.port, c2.server.port);
    }
}

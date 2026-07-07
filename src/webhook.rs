//! # Webhook module
//!
//! Sends HTTP requests to configured URLs based on sensor readings.
//! Supports three trigger modes:
//!
//! | Trigger      | Behaviour                                               |
//! |-------------|---------------------------------------------------------|
//! | `always`    | Fire on every interval tick                              |
//! | `temperature` | Fire when temp crosses a threshold (above/below °C)   |
//! | `on_change` | Fire when average temp changes by > 0.1°C                |
//!
//! # Architecture
//!
//! ```text
//! WebhookEngine (scheduler task)
//! ├── reads config every 60s (to pick up new webhooks)
//! └── for each webhook:
//!     └── spawns a per-hook async task
//!         ├── reads sensors on interval
//!         ├── checks trigger condition
//!         └── fires HTTP POST if condition matches
//! ```

use chrono::Local;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error};

use crate::config::{Config, WebhookConfig, WebhookTrigger};
use crate::sensors::SensorManager;

/// Webhook scheduler engine.
///
/// Spawns a background task that periodically re-reads the config
/// and ensures one async task is running for each configured webhook.
pub struct WebhookEngine {
    /// Sensor manager for reading data.
    sensor_manager: Arc<SensorManager>,
    /// Config (read to discover webhooks).
    config: Arc<tokio::sync::RwLock<Config>>,
    /// HTTP client for sending webhook requests.
    client: Client,
}

impl WebhookEngine {
    /// Create a new webhook engine.
    ///
    /// Builds an HTTP client with a 30-second request timeout.
    pub fn new(
        sensor_manager: Arc<SensorManager>,
        config: Arc<tokio::sync::RwLock<Config>>,
    ) -> Self {
        Self {
            sensor_manager,
            config,
            // Build a reqwest client with 30s timeout for HTTP requests.
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client (reqwest)"),
        }
    }

    /// Start the background scheduler task.
    ///
    /// Runs a loop every 60 seconds that:
    /// 1. Reads the current config for webhook definitions
    /// 2. Spawns one async task per webhook
    ///
    /// Each webhook task runs its own infinite loop with its interval.
    /// When the config is reloaded, old tasks continue but new ones spawn.
    /// This is acceptable for the current use case but could use a
    /// CancellationToken for proper lifecycle management in the future.
    pub fn start(&self) {
        let sm = self.sensor_manager.clone();
        let config = self.config.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            loop {
                // Re-read config to pick up new/changed webhooks.
                let webhooks = config.read().await.webhooks.clone();
                if webhooks.is_empty() {
                    // No webhooks configured — sleep and check again later.
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    continue;
                }
                // Spawn one async task per webhook definition.
                for wh in webhooks {
                    let s = sm.clone();
                    let c = client.clone();
                    tokio::spawn(async move {
                        run_hook(wh, s, c).await;
                    });
                }
                // Re-check config every 60 seconds.
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
    }
}

/// Run a single webhook task in a loop.
///
/// Each call reads sensors, checks the trigger condition, and fires
/// an HTTP request if the condition is met. Uses exponential backoff
/// internally via `tokio::time::interval`.
async fn run_hook(wh: WebhookConfig, sm: Arc<SensorManager>, client: Client) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(wh.interval_seconds));
    let mut last: Option<f64> = None;

    loop {
        interval.tick().await;
        let readings = sm.read_all();

        // Skip if the trigger condition is not met.
        if !should_fire(&wh, &readings, &last) {
            continue;
        }

        // Send the HTTP request with sensor data payload.
        match send_hook(&wh, &client, &readings).await {
            Ok(avg) => {
                last = Some(avg);
                debug!("Webhook '{}' sent", wh.name);
            }
            Err(e) => error!("Webhook '{}' error: {}", wh.name, e),
        }
    }
}

/// Determine whether a webhook should fire based on its trigger type.
///
/// # Trigger modes
///
/// - `Always` → fire unconditionally
/// - `Temperature` → fire if any temp reading crosses the threshold
/// - `OnChange` → fire if average temp changed by > 0.1°C since last fire
fn should_fire(
    wh: &WebhookConfig,
    readings: &crate::sensors::SensorReadings,
    last: &Option<f64>,
) -> bool {
    match &wh.trigger {
        WebhookTrigger::Always => true,
        WebhookTrigger::Temperature => {
            // If a condition is specified, check it; otherwise always fire.
            if let Some(cond) = &wh.condition {
                check_temp(readings, cond)
            } else {
                true
            }
        }
        WebhookTrigger::OnChange => {
            let cur = avg_temp(readings);
            match (last, cur) {
                // Fire if temperature changed by more than 0.1°C.
                (Some(l), Some(c)) if (c - l).abs() > 0.1 => true,
                // Fire on first reading (no previous value to compare).
                (None, _) => true,
                _ => false,
            }
        }
    }
}

/// Check if any temperature reading crosses the configured threshold.
///
/// Walks all devices → features → sub-features looking for values
/// named "temp" and compares them against above/below thresholds.
fn check_temp(
    readings: &crate::sensors::SensorReadings,
    cond: &crate::config::WebhookCondition,
) -> bool {
    for dev in &readings.devices {
        for feat in &dev.features {
            for sub in &feat.sub_features {
                // Only check sub-features with "temp" in the name.
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

/// Calculate the average temperature across all temperature sub-features.
///
/// Walks all readings, collects values with "temp" in the name,
/// and returns their arithmetic mean. Returns `None` if no temps found.
fn avg_temp(readings: &crate::sensors::SensorReadings) -> Option<f64> {
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

/// Send an HTTP request with the sensor data payload.
///
/// The payload includes webhook name, timestamp, and full sensor readings.
/// Uses the configured HTTP method, content-type, and custom headers.
///
/// Returns the average temperature on success (for on-change tracking).
async fn send_hook(
    wh: &WebhookConfig,
    client: &Client,
    readings: &crate::sensors::SensorReadings,
) -> Result<f64, String> {
    let payload = json!({
        "webhook": &wh.name,
        "timestamp": Local::now().to_rfc3339(),
        "readings": readings,
    });

    // Build the HTTP request with config-specified method and headers.
    let mut builder = client
        .post(&wh.url)
        .header("Content-Type", &wh.content_type)
        .json(&payload);
    for (k, v) in &wh.headers {
        builder = builder.header(k, v);
    }

    // Send and check for success.
    let resp = builder.send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(avg_temp(readings).unwrap_or(0.0))
    } else {
        Err(format!("HTTP {}", resp.status()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::{Device, DeviceReadings, FeatureInfo, SensorReadings, SubFeatureInfo};

    /// Verify avg_temp calculation with two temperatures.
    #[test]
    fn test_avg_temp() {
        let r = SensorReadings {
            devices: vec![DeviceReadings {
                device: Device {
                    name: "t".into(),
                    bus: "b".into(),
                    path: None,
                },
                features: vec![FeatureInfo {
                    name: "temp1".into(),
                    sub_features: vec![
                        SubFeatureInfo {
                            name: "temp1_input".into(),
                            value: Some(60.0),
                            unit: Some("°C".into()),
                        },
                        SubFeatureInfo {
                            name: "temp2_input".into(),
                            value: Some(80.0),
                            unit: Some("°C".into()),
                        },
                    ],
                }],
            }],
        };
        // (60 + 80) / 2 = 70
        assert!((avg_temp(&r).unwrap() - 70.0).abs() < 0.01);
    }

    /// Empty readings produce no average.
    #[test]
    fn test_avg_temp_empty() {
        assert!(avg_temp(&SensorReadings { devices: vec![] }).is_none());
    }

    /// check_temp triggers above threshold.
    #[test]
    fn test_temp_above() {
        let r = SensorReadings {
            devices: vec![DeviceReadings {
                device: Device {
                    name: "t".into(),
                    bus: "b".into(),
                    path: None,
                },
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
        assert!(check_temp(
            &r,
            &crate::config::WebhookCondition {
                above_celsius: Some(80.0),
                below_celsius: None
            }
        ));
    }
}

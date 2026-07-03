use chrono::Local;
use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error};

use crate::config::{Config, WebhookConfig, WebhookTrigger};
use crate::sensors::SensorManager;

pub struct WebhookEngine {
    sensor_manager: Arc<SensorManager>,
    config: Arc<tokio::sync::RwLock<Config>>,
    client: Client,
}

impl WebhookEngine {
    pub fn new(
        sensor_manager: Arc<SensorManager>,
        config: Arc<tokio::sync::RwLock<Config>>,
    ) -> Self {
        Self {
            sensor_manager,
            config,
            client: Client::builder().timeout(std::time::Duration::from_secs(30)).build().unwrap(),
        }
    }

    pub fn start(&self) {
        let sm = self.sensor_manager.clone();
        let config = self.config.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            loop {
                let webhooks = config.read().await.webhooks.clone();
                if webhooks.is_empty() {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    continue;
                }
                for wh in webhooks {
                    let s = sm.clone();
                    let c = client.clone();
                    tokio::spawn(async move {
                        run_hook(wh, s, c).await;
                    });
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
    }
}

async fn run_hook(wh: WebhookConfig, sm: Arc<SensorManager>, client: Client) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(wh.interval_seconds));
    let mut last: Option<f64> = None;

    loop {
        interval.tick().await;
        let readings = sm.read_all();

        if !should_fire(&wh, &readings, &last) {
            continue;
        }

        match send_hook(&wh, &client, &readings).await {
            Ok(avg) => { last = Some(avg); debug!("Webhook '{}' sent", wh.name); }
            Err(e) => error!("Webhook '{}' error: {}", wh.name, e),
        }
    }
}

fn should_fire(wh: &WebhookConfig, readings: &crate::sensors::SensorReadings, last: &Option<f64>) -> bool {
    match &wh.trigger {
        WebhookTrigger::Always => true,
        WebhookTrigger::Temperature => {
            if let Some(cond) = &wh.condition {
                check_temp(readings, cond)
            } else {
                true
            }
        }
        WebhookTrigger::OnChange => {
            let cur = avg_temp(readings);
            match (last, cur) {
                (Some(l), Some(c)) if (c - l).abs() > 0.1 => true,
                (None, _) => true,
                _ => false,
            }
        }
    }
}

fn check_temp(readings: &crate::sensors::SensorReadings, cond: &crate::config::WebhookCondition) -> bool {
    for chip in &readings.chips {
        for feat in &chip.features {
            for sub in &feat.sub_features {
                if sub.name.contains("temp") {
                    if let Some(v) = sub.value {
                        if let Some(above) = cond.above_celsius {
                            if v > above { return true; }
                        }
                        if let Some(below) = cond.below_celsius {
                            if v < below { return true; }
                        }
                    }
                }
            }
        }
    }
    false
}

fn avg_temp(readings: &crate::sensors::SensorReadings) -> Option<f64> {
    let mut sum = 0.0_f64;
    let mut count = 0u32;
    for chip in &readings.chips {
        for feat in &chip.features {
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
    if count > 0 { Some(sum / count as f64) } else { None }
}

async fn send_hook(wh: &WebhookConfig, client: &Client, readings: &crate::sensors::SensorReadings) -> Result<f64, String> {
    let payload = json!({
        "webhook": &wh.name,
        "timestamp": Local::now().to_rfc3339(),
        "readings": readings,
    });

    let mut builder = client
        .post(&wh.url)
        .header("Content-Type", &wh.content_type)
        .json(&payload);
    for (k, v) in &wh.headers {
        builder = builder.header(k, v);
    }

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
    use crate::sensors::{ChipInfo, ChipReadings, FeatureInfo, SensorReadings, SubFeatureInfo};

    #[test]
    fn test_avg_temp() {
        let r = SensorReadings {
            chips: vec![ChipReadings {
                chip: ChipInfo { name: "t".into(), bus: "b".into(), path: None },
                features: vec![FeatureInfo {
                    name: "temp1".into(),
                    sub_features: vec![
                        SubFeatureInfo { name: "temp1_input".into(), value: Some(60.0), unit: Some("°C".into()) },
                        SubFeatureInfo { name: "temp2_input".into(), value: Some(80.0), unit: Some("°C".into()) },
                    ],
                }],
            }],
        };
        assert!((avg_temp(&r).unwrap() - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_avg_temp_empty() {
        assert!(avg_temp(&SensorReadings { chips: vec![] }).is_none());
    }

    #[test]
    fn test_temp_above() {
        let r = SensorReadings {
            chips: vec![ChipReadings {
                chip: ChipInfo { name: "t".into(), bus: "b".into(), path: None },
                features: vec![FeatureInfo {
                    name: "temp1".into(),
                    sub_features: vec![SubFeatureInfo { name: "temp1_input".into(), value: Some(90.0), unit: Some("°C".into()) }],
                }],
            }],
        };
        assert!(check_temp(&r, &crate::config::WebhookCondition { above_celsius: Some(80.0), below_celsius: None }));
    }
}

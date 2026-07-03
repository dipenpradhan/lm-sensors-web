use axum::Json;
use axum::extract::State;
use chrono::Local;
use serde_json::json;

use crate::config::Config;
use crate::state::AppState;

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": Local::now().to_rfc3339()
    }))
}

pub async fn reload_config(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let config_path = std::env::var("CONFIG_PATH")
        .unwrap_or_else(|_| std::string::String::from("config.json"));
    let path = std::path::Path::new(&config_path);

    match Config::load(path) {
        Ok(new_config) => {
            *state.config.write().await = new_config;
            Json(json!({
                "status": "ok",
                "message": "Config reloaded successfully"
            }))
        }
        Err(e) => Json(json!({
            "status": "error",
            "message": format!("Failed to reload config: {}", e)
        })),
    }
}

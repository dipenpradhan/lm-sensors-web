//! # Health and config endpoints
//!
//! - `GET /api/health` — liveness probe (returns status + timestamp)
//! - `POST /api/reload` — reload config file at runtime

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::Local;
use serde_json::json;

use crate::config::Config;
use crate::state::AppState;

/// Liveness probe endpoint.
///
/// Returns `{"status": "ok", "timestamp": "..."}` with 200 OK.
/// Used by load balancers, health-check scripts, and Kubernetes probes.
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": Local::now().to_rfc3339()
    }))
}

/// Reload the configuration file at runtime.
///
/// Reads the config path from `AppState` (set by `main.rs`).
/// Updates the shared `RwLock<Config>` so subsequent requests
/// use the new values (webhook URLs, broadcast intervals, etc.).
///
/// Returns:
/// - 200 OK with success message on reload
/// - 500 Internal Server Error if reload failed
pub async fn reload_config(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    let path = std::path::Path::new(&state.config_path);

    match Config::load(path) {
        Ok(new_config) => {
            // Update the shared config behind the async RwLock.
            *state.config.write().await = new_config;
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ok",
                    "message": "Config reloaded successfully"
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Failed to reload config: {}", e)
            })),
        ),
    }
}

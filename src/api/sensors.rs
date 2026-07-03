use axum::Json;
use axum::extract::{Path, State};
use serde_json::json;

use crate::state::AppState;

pub async fn get_sensors(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let chips = state.sensor_manager.list_chips();
    Json(json!({ "chips": chips }))
}

pub async fn get_sensor(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.sensor_manager.get_chip(&id) {
        Some(c) => Json(json!({ "chip": c })),
        None => Json(json!({ "error": format!("Chip '{}' not found", id) })),
    }
}

pub async fn get_sensor_features(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.sensor_manager.get_chip_features(&id) {
        Some(r) => Json(json!(r)),
        None => Json(json!({ "error": format!("Chip '{}' not found", id) })),
    }
}

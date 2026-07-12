//! # Sensor device endpoints
//!
//! - `GET /api/devices` — list all devices (metadata only)
//! - `GET /api/devices/{device_id}` — get a specific device by name
//! - `GET /api/devices/{device_id}/features` — get device with all readings

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde_json::json;

use crate::state::AppState;

/// List all detected sensor devices.
///
/// Returns `{"devices": [...]}` with device metadata (name, bus, path)
/// but not the actual sensor readings. Fast — no sensor read required.
///
/// Status: 200 OK
pub async fn get_devices(State(state): State<AppState>) -> Json<serde_json::Value> {
    let devices = state.sensor_manager.list_devices();
    Json(json!({ "devices": devices }))
}

/// Get full sensor readings from all devices.
///
/// Returns `{"devices": [{"device": {...}, "features": [...]}]}`
/// with all current sensor values. This is the main endpoint used by
/// the dashboard REST fallback and external consumers.
///
/// Status: 200 OK
pub async fn get_sensors(State(state): State<AppState>) -> Json<serde_json::Value> {
    let readings = state.sensor_manager.read_all();
    Json(json!(readings))
}

/// Get a specific device by ID (partial name match).
///
/// Searches all devices and returns the first whose name contains
/// the given `id` substring. Returns metadata only.
///
/// Status: 200 OK or 404 Not Found
pub async fn get_device(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.sensor_manager.get_device(&id) {
        Some(d) => (StatusCode::OK, Json(json!({ "device": d }))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Device '{}' not found", id) })),
        ),
    }
}

/// Get a specific device with all its features and current readings.
///
/// Like `get_device` but also reads every feature/sub-feature value.
/// Used by the dashboard to display sensor values.
///
/// Status: 200 OK or 404 Not Found
pub async fn get_device_features(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match state.sensor_manager.get_device_features(&id) {
        Some(r) => (StatusCode::OK, Json(json!(r))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Device '{}' not found", id) })),
        ),
    }
}

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::api::health::{health, reload_config};
use crate::api::sensors::{get_sensors, get_sensor, get_sensor_features};
use crate::state::AppState;
use crate::websocket::WebSocketServer;

pub fn create_router(
    state: AppState,
    ws_path: Option<String>,
    ws_server: Option<&WebSocketServer>,
) -> Router {
    let mut router = Router::new()
        .route("/api/health", axum::routing::get(health))
        .route("/api/reload", axum::routing::post(reload_config))
        .route("/api/sensors", axum::routing::get(get_sensors))
        .route("/api/sensors/:chip_id", axum::routing::get(get_sensor))
        .route("/api/sensors/:chip_id/features", axum::routing::get(get_sensor_features))
        .with_state(state.clone());

    if let (Some(path), Some(ws)) = (ws_path, ws_server) {
        let handler = ws.make_handler();
        router = router.route(&path, handler);
    }

    router.layer(CorsLayer::permissive())
}

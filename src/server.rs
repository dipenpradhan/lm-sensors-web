use axum::{Router, response::Redirect};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

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

    // Serve static files at /static/
    router = router.nest_service("/static", ServeDir::new("static"));

    // WebSocket
    if let (Some(path), Some(ws)) = (ws_path, ws_server) {
        let handler = ws.make_handler();
        router = router.route(&path, handler);
    }

    // Fallback: redirect root to frontend
    router = router.fallback(axum::routing::get(index_handler));

    router.layer(CorsLayer::permissive())
}

async fn index_handler() -> Redirect {
    Redirect::to("/static/index.html")
}

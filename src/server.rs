//! # Server module
//!
//! Constructs the Axum `Router` with all REST routes, WebSocket endpoint,
//! static file serving, CORS middleware, and a fallback redirect.
//!
//! # Route table
//!
//! | Method | Path                                | Handler             |
//! |--------|-------------------------------------|---------------------|
//! | GET    | `/api/health`                       | `health`            |
//! | POST   | `/api/reload`                       | `reload_config`     |
//! | GET    | `/api/devices`                      | `get_devices`       |
//! | GET    | `/api/devices/{device_id}`          | `get_device`        |
//! | GET    | `/api/devices/{device_id}/features` | `get_device_features`|
//! | GET    | `/ws/sensors` (upgradable)          | `ws::upgrade_handler`|
//! | GET    | `/static/*`                         | `ServeDir`          |
//! | GET    | `/*` (fallback)                     | redirect to `/static/index.html`|

use axum::{Router, response::Redirect};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::api::health::{health, reload_config};
use crate::api::sensors::{get_device, get_device_features, get_devices, get_sensors};
use crate::state::AppState;
use crate::websocket::WebSocketServer;

/// Build the complete Axum router with all routes and middleware.
///
/// # Arguments
/// * `state` — shared application state (sensor manager, config, WS state)
/// * `ws_path` — WebSocket endpoint path (e.g. "/ws/sensors"); `None` if WS disabled
/// * `ws_server` — WebSocket broadcast server handle; `None` if WS disabled
///
/// # Returns
/// An Axum `Router` ready to be served.
pub fn create_router(
    state: AppState,
    ws_path: Option<String>,
    ws_server: Option<&WebSocketServer>,
) -> Router {
    let mut router = Router::new()
        // Health-check endpoint — returns status and timestamp.
        .route("/api/health", axum::routing::get(health))
        // Reload config at runtime — re-reads config.json.
        .route("/api/reload", axum::routing::post(reload_config))
        // List all sensor devices (metadata only).
        .route("/api/devices", axum::routing::get(get_devices))
        // Get full sensor readings from all devices.
        .route("/api/sensors", axum::routing::get(get_sensors))
        // Get a specific device by partial name match.
        .route("/api/devices/{device_id}", axum::routing::get(get_device))
        // Get a device with all its features and current readings.
        .route(
            "/api/devices/{device_id}/features",
            axum::routing::get(get_device_features),
        )
        // Share application state across all routes above.
        .with_state(state.clone());

    // Serve static files (HTML, CSS, JS) from the `static/` directory.
    // These are the frontend dashboard assets.
    router = router.nest_service("/static", ServeDir::new("static"));

    // WebSocket endpoint for real-time sensor broadcast.
    // Only mounted when WebSocket is enabled in config.
    if let (Some(path), Some(ws)) = (ws_path, ws_server) {
        let handler = ws.make_handler();
        router = router.route(&path, handler);
    }

    // Fallback: redirect any unmatched URL to the dashboard.
    router = router.fallback(axum::routing::get(index_handler));

    // Restrictive CORS: only allow requests from the same origin.
    // For cross-origin access, set `CORS_ORIGINS` env var to a comma-separated
    // list of allowed origins (e.g. "https://example.com,http://localhost:3000").
    let allowed_origins: Vec<String> = std::env::var("CORS_ORIGINS")
        .ok()
        .map(|v| v.split(',').map(str::to_string).collect())
        .unwrap_or_else(|| {
            // Default: allow localhost variants only.
            vec!["http://127.0.0.1".into(), "http://localhost".into()]
        });

    use axum::http::HeaderValue;
    use tower_http::cors::AllowOrigin;
    let cors = CorsLayer::new()
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([axum::http::HeaderName::from_static("content-type")])
        .allow_origin(AllowOrigin::list(
            allowed_origins
                .into_iter()
                .filter(|o| !o.is_empty())
                .filter_map(|o| o.parse::<HeaderValue>().ok()),
        ));
    router.layer(cors)
}

/// Redirect the root path to the frontend dashboard.
///
/// When a client navigates to any unknown URL, they get redirected to
/// `/static/index.html` so they can start exploring sensor data.
async fn index_handler() -> Redirect {
    Redirect::to("/static/index.html")
}

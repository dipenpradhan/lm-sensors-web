//! # Server module
//!
//! Constructs the Axum `Router` with all REST routes, WebSocket endpoint,
//! embedded static file serving, CORS middleware, and a fallback handler.
//!
//! # Route table
//!
//! | Method | Path                                | Handler             |
//! |--------|-------------------------------------|---------------------|
//! | GET    | `/api/health`                       | `health`            |
//! | POST   | `/api/reload`                       | `reload_config`     |
//! | GET    | `/api/version`                      | `version`           |
//! | GET    | `/api/devices`                      | `get_devices`       |
//! | GET    | `/api/devices/{device_id}`          | `get_device`        |
//! | GET    | `/api/devices/{device_id}/features` | `get_device_features`|
//! | GET    | `/ws/sensors` (upgradable)          | `ws::upgrade_handler`|
//! | GET    | `/`                                 | embedded dashboard  |
//! | GET    | `/*` (fallback)                     | embedded dashboard  |

use axum::{Router, response::Html};
use axum::http::HeaderValue;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::api::health::{health, reload_config};
use crate::api::sensors::{get_device, get_device_features, get_devices, get_sensors};
use crate::api::version::version;
use crate::state::AppState;
use crate::websocket::WebSocketServer;

// ── Embedded dashboard ──────────────────────────────────────
// The build script (`build_dashboard.py`) minifies CSS/JS and inlines
// everything into a single `bundled.html`. This is embedded at compile
// time — zero external asset requests at runtime.
const DASHBOARD_HTML: &str = include_str!("../static/bundled.html");

/// Serve the dashboard (embedded, self-contained HTML).
async fn serve_dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

/// Build the complete Axum router with all routes and middleware.
///
/// # Arguments
/// * `state` — shared application state (sensor manager, config, WS state)
/// * `ws_path` — WebSocket endpoint path (e.g. "/ws/sensors"); `None` if WS disabled
/// * `ws_server` — WebSocket broadcast server handle; `None` if WS disabled
/// * `host` — bind address (used to compute CORS origins)
/// * `port` — listen port (used to compute CORS origins)
///
/// # Returns
/// An Axum `Router` ready to be served.
pub fn create_router(
    state: AppState,
    ws_path: Option<String>,
    ws_server: Option<&WebSocketServer>,
    host: &str,
    port: u16,
) -> Router {
    let mut router = Router::new()
        .route("/api/health", axum::routing::get(health))
        .route("/api/reload", axum::routing::post(reload_config))
        .route("/api/version", axum::routing::get(version))
        .route("/api/devices", axum::routing::get(get_devices))
        .route("/api/sensors", axum::routing::get(get_sensors))
        .route("/api/devices/{device_id}", axum::routing::get(get_device))
        .route(
            "/api/devices/{device_id}/features",
            axum::routing::get(get_device_features),
        )
        .route("/", axum::routing::get(serve_dashboard))
        .with_state(state.clone());

    // WebSocket endpoint for real-time sensor broadcast.
    if let (Some(path), Some(ws)) = (ws_path, ws_server) {
        let handler = ws.make_handler();
        router = router.route(&path, handler);
    }

    // Fallback: serve the dashboard for any unknown path.
    router = router.fallback(axum::routing::get(serve_dashboard));

    // CORS: allow origins based on the bound host/port.
    let cors = build_cors_layer(host, port);
    router.layer(cors)
}

/// Build the CORS layer based on the configured host and port.
///
/// If bound to `0.0.0.0` or `::` (all interfaces), allows any origin.
/// Otherwise, allows only the specific host:port that was configured.
/// If `CORS_ORIGINS` env var is set, those explicit origins are used instead.
fn build_cors_layer(host: &str, port: u16) -> CorsLayer {
    // Check for explicit env var override first.
    if let Ok(origins) = std::env::var("CORS_ORIGINS") {
        let list: Vec<HeaderValue> = origins
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if !list.is_empty() {
            return CorsLayer::new()
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([axum::http::HeaderName::from_static("content-type")])
                .allow_origin(AllowOrigin::list(list));
        }
    }

    // Bound to all interfaces — allow any origin.
    if host == "0.0.0.0" || host == "::" {
        tracing::info!("CORS: allowing any origin on port {}", port);
        return CorsLayer::permissive();
    }

    // Allow the specific host:port and localhost variants.
    let mut list = Vec::new();
    let origin_with_port = format!("http://{}:{}", host, port);
    let origin_no_port = format!("http://{}", host);

    for origin in [origin_with_port, origin_no_port] {
        if let Ok(h) = origin.parse::<HeaderValue>() {
            list.push(h);
        }
    }

    // Also allow localhost/127.0.0.1 if we're binding to a local address.
    if host == "localhost" || host == "127.0.0.1" {
        for alt in ["http://localhost:47890", "http://127.0.0.1:47890"] {
            if let Ok(h) = alt.parse::<HeaderValue>() {
                if !list.contains(&h) {
                    list.push(h);
                }
            }
        }
    }

    CorsLayer::new()
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([axum::http::HeaderName::from_static("content-type")])
        .allow_origin(AllowOrigin::list(list))
}
//! # Application state
//!
//! Shared state struct passed to Axum routes via `State<T>` extractor.
//! Clonable via `Arc` so each route handler owns its own lightweight handle.
//!
//! # Thread safety
//!
//! All inner handles are `Arc<T>` or `Arc<RwLock<T>>`, so `AppState` itself
//! can be freely cloned and shared across async tasks without additional locking.

use std::sync::Arc;

use crate::config::Config;
use crate::sensors::SensorManager;
use crate::websocket::WebSocketState;

/// Shared application state.
///
/// Cloned into every route handler via Axum's `State` extractor.
/// The actual data lives behind `Arc` pointers so cloning is cheap.
#[derive(Clone)]
pub struct AppState {
    /// Sensor manager — wraps `lm-sensors` crate for safe concurrent access.
    pub sensor_manager: Arc<SensorManager>,

    /// Runtime config behind an async `RwLock` for re-load support.
    pub config: Arc<tokio::sync::RwLock<Config>>,

    /// Optional WebSocket broadcast state (may be `None` if WS is disabled).
    /// Kept for future use — WS client filtering by subscription.
    #[allow(dead_code)]
    pub ws_state: Option<Arc<WebSocketState>>,

    /// Path to the config file (used by `reload_config` endpoint).
    /// Stored in state instead of relying on an env var.
    pub config_path: String,
}

use std::sync::Arc;

use crate::config::Config;
use crate::sensors::SensorManager;
use crate::websocket::WebSocketState;

#[derive(Clone)]
pub struct AppState {
    pub sensor_manager: Arc<SensorManager>,
    pub config: Arc<tokio::sync::RwLock<Config>>,
    pub ws_state: Option<Arc<WebSocketState>>,
}

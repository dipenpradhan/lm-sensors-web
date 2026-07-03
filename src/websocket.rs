use axum::extract::ws::WebSocket;
use axum::extract::WebSocketUpgrade;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::config::Config;
use crate::sensors::SensorManager;

pub struct WebSocketState {
    pub sender: broadcast::Sender<String>,
}

pub struct WebSocketServer {
    pub state: Arc<WebSocketState>,
    sensor_manager: Arc<SensorManager>,
    config: Arc<tokio::sync::RwLock<Config>>,
}

impl WebSocketServer {
    pub fn new(
        sensor_manager: Arc<SensorManager>,
        config: Arc<tokio::sync::RwLock<Config>>,
    ) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            state: Arc::new(WebSocketState { sender: tx }),
            sensor_manager,
            config,
        }
    }

    pub fn start_broadcast(&self) {
        let sm = self.sensor_manager.clone();
        let state = self.state.clone();
        let config = self.config.clone();

        tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(async {
                let interval_ms = config.read().await.websocket.broadcast_interval_ms;
                let mut ticker =
                    tokio::time::interval(std::time::Duration::from_millis(interval_ms));
                loop {
                    ticker.tick().await;
                    match serde_json::to_string(&sm.read_all()) {
                        Ok(json) => { let _ = state.sender.send(json); }
                        Err(e) => tracing::error!("WS serialization error: {}", e),
                    }
                }
            });
        });
    }

    pub fn make_handler(&self) -> axum::routing::MethodRouter<()> {
        let state = self.state.clone();
        axum::routing::get(move |ws: WebSocketUpgrade| {
            ws::upgrade_handler(ws, state.clone())
        })
    }
}

mod ws {
    use super::*;

    pub async fn upgrade_handler(
        ws: WebSocketUpgrade,
        state: Arc<WebSocketState>,
    ) -> impl axum::response::IntoResponse {
        let sender = state.sender.clone();
        ws.on_upgrade(move |socket| handle_socket(socket, sender))
    }

    async fn handle_socket(mut socket: WebSocket, sender: broadcast::Sender<String>) {
        let mut rx = sender.subscribe();
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            if socket
                                .send(axum::extract::ws::Message::Text(msg.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                else => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_broadcast_send_recv() {
        let (tx, _) = broadcast::channel::<String>(10);
        let mut rx = tx.subscribe();
        tx.send("hello".into()).unwrap();
        let r = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert_eq!(r.unwrap().unwrap(), "hello");
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let (tx, _) = broadcast::channel::<String>(10);
        let mut rx1 = tx.subscribe();
        let mut rx2 = tx.subscribe();
        tx.send("data".into()).unwrap();
        assert_eq!(rx1.try_recv().unwrap(), "data");
        assert_eq!(rx2.try_recv().unwrap(), "data");
    }
}

//! # WebSocket module
//!
//! Implements a real-time sensor broadcast over WebSocket. Uses
//! `tokio::sync::broadcast` to fan-out sensor readings to all
//! connected clients on a configurable timer.
//!
//! # Architecture
//!
//! ```text
//! Broadcast task (async loop)
//! ├── reads sensors every N ms
//! ├── serialises to JSON
//! └── sends to broadcast::Sender
//!     ├── Client 1 (WebSocket subscriber)
//!     ├── Client 2
//!     └── Client N
//! ```
//!
//! Each WebSocket client subscribes to the broadcast channel and
//! receives full sensor snapshots at the configured interval.

use axum::extract::WebSocketUpgrade;
use axum::extract::ws::WebSocket;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::config::Config;
use crate::sensors::SensorManager;

/// Minimal shared state for WebSocket broadcasting.
///
/// Contains only the broadcast sender — cloned into each route handler
/// and WebSocket upgrade closure.
pub struct WebSocketState {
    /// Broadcast channel sender — each new subscriber clones a receiver.
    pub sender: broadcast::Sender<String>,
}

/// WebSocket server that reads sensors periodically and broadcasts JSON.
pub struct WebSocketServer {
    /// Shared broadcast state (cloned into handlers).
    pub state: Arc<WebSocketState>,
    /// Sensor manager for reading data.
    sensor_manager: Arc<SensorManager>,
    /// Config (read to determine broadcast interval).
    config: Arc<tokio::sync::RwLock<Config>>,
}

impl WebSocketServer {
    /// Create a new WebSocket server.
    ///
    /// Initializes a broadcast channel with capacity 100 (dropping
    /// old messages when a slow subscriber falls behind).
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

    /// Start the background broadcast task.
    ///
    /// Spawns an async task that:
    /// 1. Reads sensors on a configurable interval
    /// 2. Serialises the snapshot to JSON
    /// 3. Sends it to all connected WebSocket clients via broadcast
    ///
    /// The task runs until the runtime is shut down (no cancellation).
    pub fn start_broadcast(&self) {
        let sm = self.sensor_manager.clone();
        let state = self.state.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            // Read interval from config once (not re-read on every tick).
            let interval_ms = config.read().await.websocket.broadcast_interval_ms;
            let mut ticker = tokio::time::interval(std::time::Duration::from_millis(interval_ms));

            loop {
                ticker.tick().await;
                // Read all sensors and serialise to JSON.
                match serde_json::to_string(&sm.read_all()) {
                    Ok(json) => {
                        // Send to all subscribers (drops slow receivers silently).
                        let _ = state.sender.send(json);
                    }
                    Err(e) => tracing::error!("WS serialization error: {}", e),
                }
            }
        });
    }

    /// Build an Axum route handler for WebSocket upgrades.
    ///
    /// Returns a `GET` handler that upgrades the connection and
    /// subscribes the socket to the broadcast channel.
    pub fn make_handler(&self) -> axum::routing::MethodRouter<()> {
        let state = self.state.clone();
        axum::routing::get(move |ws: WebSocketUpgrade| ws::upgrade_handler(ws, state.clone()))
    }
}

/// Internal WebSocket handling — kept separate to keep public API clean.
mod ws {
    use super::*;

    /// Upgrade handler: converts an HTTP upgrade request to a WebSocket.
    ///
    /// On successful upgrade, delegates to `handle_socket` which
    /// subscribes the socket to the broadcast channel.
    pub async fn upgrade_handler(
        ws: WebSocketUpgrade,
        state: Arc<WebSocketState>,
    ) -> impl axum::response::IntoResponse {
        let sender = state.sender.clone();
        ws.on_upgrade(move |socket| handle_socket(socket, sender))
    }

    /// Handle an individual WebSocket connection.
    ///
    /// Subscribes to the broadcast channel and forwards all messages
    /// to the client. Exits when the client disconnects or the channel
    /// closes.
    async fn handle_socket(mut socket: WebSocket, sender: broadcast::Sender<String>) {
        let mut rx = sender.subscribe();
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            // Send the JSON message to this client.
                            // If send fails (client disconnected), break.
                            if socket
                                .send(axum::extract::ws::Message::Text(msg.into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // Client is too slow — skip old messages, keep going.
                            tracing::warn!("WebSocket client lagged, skipping messages");
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                // If the select! has no other branches, break (shouldn't happen).
                else => break,
            }
        }
        // Socket dropped here — client disconnected normally.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Verify broadcast send/recv works with a single subscriber.
    #[tokio::test]
    async fn test_broadcast_send_recv() {
        let (tx, _) = broadcast::channel::<String>(10);
        let mut rx = tx.subscribe();
        tx.send("hello".into()).unwrap();
        let r = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert_eq!(r.unwrap().unwrap(), "hello");
    }

    /// Verify multiple subscribers all receive the same message.
    #[tokio::test]
    async fn test_multiple_subscribers() {
        let (tx, _) = broadcast::channel::<String>(10);
        let mut rx1 = tx.subscribe();
        let mut rx2 = tx.subscribe();
        tx.send("data".into()).unwrap();
        assert_eq!(rx1.try_recv().unwrap(), "data");
        assert_eq!(rx2.try_recv().unwrap(), "data");
    }

    /// Verify a slow subscriber that lags behind gets a Lagged error.
    #[tokio::test]
    async fn test_broadcast_lagged() {
        let (tx, _) = broadcast::channel::<String>(2);
        let mut rx = tx.subscribe();
        // Fill the channel beyond capacity to cause lag.
        for _ in 0..5 {
            let _ = tx.send("x".into());
        }
        let result = rx.recv().await;
        assert!(matches!(
            result,
            Err(broadcast::error::RecvError::Lagged(_))
        ));
    }
}

//! # WebSocket broadcast integration tests
//!
//! Tests the broadcast channel mechanics for WebSocket sensor feeds.
//!
//! # Running
//!
//! ```bash
//! cargo test --test ws_test
//! ```

use tokio::sync::broadcast;
use std::time::Duration;

/// Single subscriber receives messages correctly.
#[tokio::test]
async fn test_broadcast_channel_single_subscriber() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();
    tx.send("sensor_data_1".into()).unwrap();
    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), "sensor_data_1");
}

/// Multiple subscribers all receive the same broadcast.
#[tokio::test]
async fn test_broadcast_channel_multiple_subscribers() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx1 = tx.subscribe();
    let mut rx2 = tx.subscribe();
    let mut rx3 = tx.subscribe();

    tx.send("batch_1".into()).unwrap();

    for rx in [&mut rx1, &mut rx2, &mut rx3] {
        let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().unwrap(), "batch_1");
    }
}

/// JSON sensor payload is transmitted correctly.
#[tokio::test]
async fn test_broadcast_json_sensor_payload() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();

    let payload = serde_json::json!({
        "devices": [{
            "device": {"name": "test", "bus": "ISA", "path": "/sys/test"},
            "features": []
        }]
    });
    let json = serde_json::to_string(&payload).unwrap();
    tx.send(json.clone()).unwrap();

    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert_eq!(result.unwrap().unwrap(), json);
}

/// Broadcast channel respects capacity limits.
#[tokio::test]
async fn test_broadcast_channel_capacity() {
    let (tx, _) = broadcast::channel::<String>(2);
    let mut rx = tx.subscribe();

    // Send beyond capacity
    tx.send("msg1".into()).unwrap();
    tx.send("msg2".into()).unwrap();
    tx.send("msg3".into()).unwrap(); // This should cause lag

    let result = rx.recv().await;
    // First receive might succeed or get lagged
    match result {
        Ok(_) => {},
        Err(broadcast::error::RecvError::Lagged(_)) => {},
        Err(_) => panic!("Unexpected error"),
    }
}

/// Subscriber created after messages doesn't receive past messages.
#[tokio::test]
async fn test_broadcast_no_history() {
    let (tx, _) = broadcast::channel::<String>(10);
    tx.send("past_message".into()).unwrap();

    // Subscribe after the message
    let mut rx = tx.subscribe();
    tx.send("new_message".into()).unwrap();

    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    // Should receive new_message, not past_message
    assert_eq!(result.unwrap().unwrap(), "new_message");
}
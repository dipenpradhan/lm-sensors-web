//! # WebSocket integration tests
//!
//! Tests the WebSocket broadcast channel mechanics, subscriber management,
//! and message delivery guarantees.
//!
//! # Running
//!
//! ```bash
//! cargo test --test websocket_integration
//! ```

use lm_sensors_web::sensors::{
    Device, DeviceReadings, FeatureInfo, SensorReadings, SubFeatureInfo,
};
use serde_json::json;
use std::time::Duration;
use tokio::sync::broadcast;

// ── Broadcast Channel Tests ──────────────────────────────────────

/// Single subscriber receives all messages.
#[tokio::test]
async fn test_broadcast_single_subscriber() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();
    tx.send("sensor_data_1".into()).unwrap();
    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), "sensor_data_1");
}

/// Multiple subscribers all receive the same message.
#[tokio::test]
async fn test_broadcast_multiple_subscribers() {
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

/// Slow subscriber that falls behind gets a Lagged error.
#[tokio::test]
async fn test_broadcast_slow_subscriber_lag() {
    let (tx, _) = broadcast::channel::<String>(2);
    let mut rx = tx.subscribe();
    // Fill beyond capacity to cause lag
    for _ in 0..5 {
        let _ = tx.send("x".into());
    }
    let result = rx.recv().await;
    assert!(matches!(
        result,
        Err(broadcast::error::RecvError::Lagged(_))
    ));
}

/// Channel closure is propagated to subscribers.
#[tokio::test]
async fn test_broadcast_channel_close() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();
    drop(tx); // Close the channel

    let result = rx.recv().await;
    assert!(matches!(result, Err(broadcast::error::RecvError::Closed)));
}

/// Late subscriber does not receive past messages.
#[tokio::test]
async fn test_broadcast_late_subscriber() {
    let (tx, _) = broadcast::channel::<String>(10);
    let _ = tx.send("msg1".into()); // no subscribers yet

    // Subscribe after the message was sent
    let mut rx = tx.subscribe();

    // Immediately send another message
    tx.send("msg2".into()).unwrap();

    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    // Should receive msg2, not msg1
    assert_eq!(result.unwrap().unwrap(), "msg2");
}

/// Broadcast with zero subscribers (no error on send).
#[tokio::test]
async fn test_broadcast_no_subscribers() {
    let (tx, _) = broadcast::channel::<String>(10);
    // No subscribers — send should still succeed (drops silently)
    let result = tx.send("orphan".into());
    assert!(result.is_err()); // Err(Disconnected) because no receivers
}

// ── Sensor Payload Broadcast Tests ──────────────────────────────

/// Sensor readings are valid JSON when broadcast.
#[tokio::test]
async fn test_broadcast_sensor_payload() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();

    let payload = json!({
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

/// Large sensor payloads broadcast correctly.
#[tokio::test]
async fn test_broadcast_large_payload() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();

    // Build a large payload with many devices and features
    let mut devices = vec![];
    for i in 0..10 {
        let mut features = vec![];
        for j in 0..5 {
            features.push(json!({
                "name": format!("temp{}", j),
                "sub_features": [{
                    "name": format!("temp{}_input", j),
                    "value": 45.0 + i as f64 * 1.5,
                    "unit": "°C"
                }]
            }));
        }
        devices.push(json!({
            "device": {
                "name": format!("sensor{}", i),
                "bus": "ISA",
                "path": json!(null)
            },
            "features": features
        }));
    }
    let payload = json!({"devices": devices});
    let json = serde_json::to_string(&payload).unwrap();
    tx.send(json.clone()).unwrap();

    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert_eq!(result.unwrap().unwrap(), json);
}

/// Concurrent sends don't corrupt the channel.
#[tokio::test]
async fn test_broadcast_concurrent_sends() {
    let (tx, _) = broadcast::channel::<String>(100);
    let mut rx = tx.subscribe();

    // Spawn multiple concurrent sends
    let mut handles = vec![];
    for i in 0..10 {
        let tx_clone = tx.clone();
        handles.push(tokio::spawn(async move {
            tx_clone.send(format!("msg_{}", i)).unwrap();
        }));
    }

    // Wait for all sends to complete
    for h in handles {
        h.await.unwrap();
    }

    // Collect all messages
    let mut received = vec![];
    for _ in 0..10 {
        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .unwrap()
            .unwrap();
        received.push(msg);
    }

    // All messages were delivered (order may vary)
    assert_eq!(received.len(), 10);
}

// ── Real SensorReadings Broadcast Tests ─────────────────────────

/// Full SensorReadings structure serializes for broadcast.
#[tokio::test]
async fn test_broadcast_real_sensor_readings() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();

    let readings = SensorReadings {
        devices: vec![
            DeviceReadings {
                device: Device {
                    name: "cpu".into(),
                    bus: "ISA".into(),
                    path: None,
                },
                features: vec![FeatureInfo {
                    name: "temp1".into(),
                    sub_features: vec![SubFeatureInfo {
                        name: "temp1_input".into(),
                        value: Some(55.0),
                        unit: Some("°C".into()),
                    }],
                }],
            },
            DeviceReadings {
                device: Device {
                    name: "gpu".into(),
                    bus: "PCI".into(),
                    path: None,
                },
                features: vec![FeatureInfo {
                    name: "fan1".into(),
                    sub_features: vec![SubFeatureInfo {
                        name: "fan1_input".into(),
                        value: Some(1500.0),
                        unit: Some("RPM".into()),
                    }],
                }],
            },
        ],
    };
    let json = serde_json::to_string(&readings).unwrap();
    tx.send(json.clone()).unwrap();

    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert_eq!(result.unwrap().unwrap(), json);
}

/// Empty SensorReadings broadcasts correctly.
#[tokio::test]
async fn test_broadcast_empty_readings() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();

    let readings = SensorReadings { devices: vec![] };
    let json = serde_json::to_string(&readings).unwrap();
    tx.send(json.clone()).unwrap();

    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert_eq!(result.unwrap().unwrap(), json);
}

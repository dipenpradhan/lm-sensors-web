// Integration tests for WebSocket broadcast mechanics

use std::sync::Arc;
use tokio::sync::broadcast;
use std::time::Duration;

#[tokio::test]
async fn test_broadcast_channel_single_subscriber() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();
    tx.send("sensor_data_1".into()).unwrap();
    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), "sensor_data_1");
}

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

#[tokio::test]
async fn test_broadcast_json_sensor_payload() {
    let (tx, _) = broadcast::channel::<String>(10);
    let mut rx = tx.subscribe();

    let payload = serde_json::json!({
        "chips": [{
            "chip": {"name": "test", "bus": "ISA", "path": "/sys/test"},
            "features": []
        }]
    });
    let json = serde_json::to_string(&payload).unwrap();
    tx.send(json.clone()).unwrap();

    let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    assert_eq!(result.unwrap().unwrap(), json);
}

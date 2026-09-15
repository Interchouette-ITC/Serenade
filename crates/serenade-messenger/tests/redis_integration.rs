//! Redis integration for `serenade-messenger` (requires `REDIS_URL`).

#![cfg(feature = "redis")]

use serenade_messenger::{RedisTransport, RedisTransportConfig, WireEnvelope};

fn redis_url() -> Option<String> {
    std::env::var("REDIS_URL")
        .ok()
        .filter(|value| !value.is_empty())
}

#[tokio::test]
async fn redis_transport_send_receive_fifo() {
    let Some(url) = redis_url() else {
        eprintln!("REDIS_URL unset; skipping redis messenger test");
        return;
    };
    let list_key = format!(
        "serenade:messenger:test:{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    );
    let transport = RedisTransport::connect(
        RedisTransportConfig::new(url)
            .with_list_key(list_key)
            .with_pool_max_size(2),
    )
    .expect("connect");
    assert!(transport.is_empty().expect("empty"));
    transport
        .send_wire(WireEnvelope::new("demo.a", b"one").expect("a"))
        .await
        .expect("send a");
    transport
        .send_wire(WireEnvelope::new("demo.b", b"two").expect("b"))
        .await
        .expect("send b");
    assert_eq!(transport.len().expect("len"), 2);
    let first = transport
        .receive_wire()
        .await
        .expect("recv")
        .expect("first");
    assert_eq!(first.message_name(), "demo.a");
    assert_eq!(first.payload(), b"one");
    let second = transport
        .receive_wire()
        .await
        .expect("recv")
        .expect("second");
    assert_eq!(second.message_name(), "demo.b");
    assert!(transport.receive_wire().await.expect("recv").is_none());
}

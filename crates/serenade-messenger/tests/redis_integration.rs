//! Redis integration for `serenade-messenger` (requires a live Redis).

#![cfg(feature = "redis")]

use std::time::Duration;

use redis::Commands;
use serenade_messenger::{MessengerError, RedisTransport, RedisTransportConfig, WireEnvelope};

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_owned())
}

fn unique_list_key(label: &str) -> String {
    format!(
        "serenade:messenger:test:{label}:{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    )
}

fn connect(list_key: &str) -> RedisTransport {
    RedisTransport::connect(
        RedisTransportConfig::new(redis_url())
            .with_list_key(list_key)
            .with_pool_max_size(2)
            .with_connection_timeout(Duration::from_secs(2))
            .with_warm_pool(false),
    )
    .expect("connect")
}

#[test]
fn config_builders_and_empty_list_key() {
    let config = RedisTransportConfig::new("redis://127.0.0.1:6379/0")
        .with_list_key("serenade:messenger:cfg")
        .with_pool_max_size(0)
        .with_connection_timeout(Duration::from_millis(500))
        .with_warm_pool(true);
    assert_eq!(config.pool_max_size, 1);
    assert!(config.warm_pool);
    assert_eq!(config.connection_timeout, Duration::from_millis(500));

    let result = RedisTransport::connect(RedisTransportConfig::new(redis_url()).with_list_key(""));
    assert!(matches!(result, Err(MessengerError::Transport { .. })));
}

#[test]
fn invalid_url_and_warm_pool_down_host_fail_connect() {
    assert!(matches!(
        RedisTransport::connect(RedisTransportConfig::new("not-a-redis-url")),
        Err(MessengerError::Transport { .. })
    ));
    let config = RedisTransportConfig::new("redis://127.0.0.1:1/0")
        .with_pool_max_size(1)
        .with_connection_timeout(Duration::from_millis(200))
        .with_warm_pool(true);
    assert!(RedisTransport::connect(config).is_err());
}

#[tokio::test]
async fn redis_transport_send_receive_fifo_and_list_key() {
    let list_key = unique_list_key("fifo");
    let transport = connect(&list_key);
    assert_eq!(transport.list_key(), list_key);
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
    assert!(transport.is_empty().expect("empty again"));
}

#[tokio::test]
async fn warm_pool_and_corrupt_frame_decode_error() {
    let list_key = unique_list_key("warm");
    let transport = RedisTransport::connect(
        RedisTransportConfig::new(redis_url())
            .with_list_key(&list_key)
            .with_warm_pool(true)
            .with_pool_max_size(2),
    )
    .expect("warm redis");
    let client = redis::Client::open(redis_url()).expect("client");
    let mut conn = client.get_connection().expect("conn");
    let _: () = conn.rpush(&list_key, b"not-a-frame").expect("rpush");
    let err = transport.receive_wire().await.expect_err("bad frame");
    assert!(matches!(err, MessengerError::Transport { .. }));
}

#[tokio::test]
async fn wrongtype_key_maps_command_errors() {
    let list_key = unique_list_key("wrongtype");
    let transport = connect(&list_key);
    let client = redis::Client::open(redis_url()).expect("client");
    let mut conn = client.get_connection().expect("conn");
    let _: () = conn.set(&list_key, "string-not-list").expect("set");
    assert!(matches!(
        transport.len(),
        Err(MessengerError::Transport { .. })
    ));
    assert!(matches!(
        transport
            .send_wire(WireEnvelope::new("demo", b"x").expect("wire"))
            .await,
        Err(MessengerError::Transport { .. })
    ));
    assert!(matches!(
        transport.receive_wire().await,
        Err(MessengerError::Transport { .. })
    ));
    let _: () = conn.del(&list_key).expect("cleanup");
}

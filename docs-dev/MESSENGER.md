# Messenger

Sync bus + async transports live in **`serenade-messenger`** ([#11](https://github.com/Interchouette-ITC/Serenade/issues/11), Redis wire [#209](https://github.com/Interchouette-ITC/Serenade/issues/209)).

## Sync bus

| Type | Role |
| --- | --- |
| `Message` / `Command` / `Event` | Stable `NAME` + marker traits |
| `MessageBus` | Register handlers; `dispatch_command` / `dispatch_event` |
| `Middleware` | Logging / validation layers |

## In-process async adapter

| Type | Role |
| --- | --- |
| `Envelope` | Type-erased `Any` payload (process-local only) |
| `Transport` | Async `send` contract |
| `InMemoryTransport` | FIFO queue for tests / local workers |

## Durable Redis wire (feature `redis`)

`Envelope` cannot cross the wire (`Any`). Durable backends use:

| Type | Role |
| --- | --- |
| `WireEnvelope` | Message name + opaque payload bytes (app owns serialization) |
| `RedisTransport` | RPUSH send / LPOP receive on a Redis list |
| `RedisTransportConfig` | URL, list key, pool options |

```rust
use serenade_messenger::{RedisTransport, RedisTransportConfig, WireEnvelope};

let transport = RedisTransport::connect(
    RedisTransportConfig::new("redis://127.0.0.1:6379/0")
        .with_list_key("serenade:messenger:default"),
)?;
let frame = WireEnvelope::new("order.place", serde_json::to_vec(&payload)?)?;
transport.send_wire(frame).await?;
if let Some(next) = transport.receive_wire().await? {
    // decode next.payload() in the worker
}
```

## Related

- Kernel overview: [KERNEL.md](KERNEL.md)
- Scheduler Messenger bridge: [SCHEDULER.md](SCHEDULER.md)

# Notifier

SMS and push channels live in **`serenade-notifier`** ([#230](https://github.com/Interchouette-ITC/Serenade/issues/230)). Email stays in [`MAILER.md`](MAILER.md).

## Types

| Type | Role |
| --- | --- |
| `Channel` | `Sms` or `Push` |
| `SmsMessage` / `PushMessage` | Channel payloads |
| `Notification` | `Sms(...)` or `Push(...)` enum + constructors |
| `NotifierError` | Validation / transport failures |

## Transports

| Transport | Role |
| --- | --- |
| `NullTransport` | Validates then discards (default DI notifier) |
| `MemoryTransport` | Records sends for tests / local |
| `SmsOnlyTransport` | Accepts SMS; rejects push with `UnsupportedChannel` |

All implement `Transport` with `supports` + sync `send`. Vendor SDKs are **not** mandatory deps; apps wrap their SMS/push providers behind `Transport`.

## DI

`FrameworkExtension` adds `RegisterDefaultNotifierPass`, which registers service id `notifier` (`DEFAULT_NOTIFIER_SERVICE`) tagged `notifier.transport` with a `NullTransport` when missing. Resolve `NotifierService` and call `send`.

## Example

```rust
use serenade_notifier::{MemoryTransport, Notification, NullTransport, Transport};

NullTransport::new().send(&Notification::sms("+15551212", "Your code is 1234"))?;
NullTransport::new().send(&Notification::push("device-token", "Hi", "Order shipped"))?;

let memory = MemoryTransport::new();
memory.send(&Notification::sms("+1", "ping"))?;
assert_eq!(memory.sent().len(), 1);
```

## Related

- [MAILER.md](MAILER.md) - email channel
- [KERNEL.md](KERNEL.md) - framework component index

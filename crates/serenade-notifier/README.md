# serenade-notifier

SMS and push notification channels (Symfony Notifier shaped).

Types: `Notification`, `Channel`, `SmsMessage`, `PushMessage`. Transports:
`NullTransport`, `MemoryTransport`, `SmsOnlyTransport`. `RegisterDefaultNotifierPass`
wires the default notifier service into the DI container.

Part of [Serenade](https://github.com/Interchouette-ITC/Serenade); most applications
depend on the [`serenade-framework`](https://crates.io/crates/serenade-framework) facade.
API documentation: <https://docs.rs/serenade-notifier>.

Licensed under Apache-2.0.

---
title: Build your first bundle application
description: Run Serenade's canonical app, trace its bundle and extension, then customize its configuration.
---

This tutorial follows the real [`serenade-demo-app`](https://github.com/Interchouette-ITC/Serenade/tree/dev/examples/demo-app)
workspace package. It teaches the current bundle composition model; it does not generate a
separate project.

## 1. Prepare the workspace

Follow [installation](../../first-steps/installation/) to install the Rust toolchain and clone
Serenade. From the repository root, run the workspace tests:

```sh
make test
```

## 2. Run the application

Run the canonical composition sample:

```sh
make demo
```

It boots `DemoBundle` and `FrameworkBundle`, resolves the greeting service, dispatches
`demo.ready`, and prints the registered routes. This is a console demonstration of the kernel
and bundle APIs, not a listening web server.

The application entry point is
[`examples/demo-app/src/main.rs`](https://github.com/Interchouette-ITC/Serenade/blob/dev/examples/demo-app/src/main.rs).
It creates an `App`, registers both bundles, boots the kernel, builds the dependency-injection
container, and shuts the app down after the demonstration.

## 3. See a bundle provide a service

`DemoBundle` declares a dependency on Serenade's framework bundle. Its `DemoExtension` owns the
`demo` package alias and registers the `demo.greeting` service:

```rust
impl BundleInterface for DemoBundle {
    fn name(&self) -> &'static str {
        "demo"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[FRAMEWORK_BUNDLE]
    }
}
```

The extension reads its parameter from configuration and exposes the greeting through the
container. The full implementation also registers console commands and an event subscriber:
[`bundle.rs`](https://github.com/Interchouette-ITC/Serenade/blob/dev/examples/demo-app/src/bundle.rs).

## 4. Change the configuration

The sample's base package configuration is
[`config/packages/demo.toml`](https://github.com/Interchouette-ITC/Serenade/blob/dev/examples/demo-app/config/packages/demo.toml):

```toml
[demo]
name = "serenade"
```

The default `dev` environment overlays that value in
[`config/packages/dev/demo.toml`](https://github.com/Interchouette-ITC/Serenade/blob/dev/examples/demo-app/config/packages/dev/demo.toml)
with `name = "serenade-dev"`.

Run the console command:

```sh
make console ARGS='demo:hello'
```

With the default `APP_ENV=dev`, it prints `hello serenade-dev`. Change `name` to `world` in the
`dev` overlay, run the same command again, and verify it prints `hello world`. The command obtains
the greeting from the container; it does not construct the message itself.

## 5. Inspect the other contributions

- `DemoBundle` implements `RouteLoader` and adds `/healthz` and `/about` to a route collection.
  The sample prints those routes; it does not start an HTTP listener.
- `DemoExtension` registers `demo:hello`, `demo:slug`, `demo:workflow`, and a subscriber for
  `demo.ready`.
- `main.rs` dispatches `demo.ready` after boot, then shuts down the kernel.

The [bundle guide](../../concepts/bundle-extensions/) explains lifecycle, extension points,
registration, and public API boundaries. To run an actual web example, continue with
[choosing a web mode](../../first-steps/choose-your-mode/).

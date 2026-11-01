---
title: Run the demo application
description: Boot the canonical Serenade bundle sample and inspect what it demonstrates.
---

The `demo-app` workspace package is the canonical small example of Serenade composition. It
registers a product bundle and the framework bundle, loads package configuration, resolves a
service, dispatches an event, and prints its routes.

For a step-by-step walkthrough and a small configuration exercise, continue to the
[first bundle application tutorial](../../tutorials/first-bundle-app/).

From the repository root, run:

```sh
make demo
```

This is equivalent to:

```sh
cargo run -p serenade-demo-app --bin serenade-demo-app
```

The output includes registered bundles, a greeting, event subscribers, and routes. This is a
console sample, not a web server or a generated application.

## Follow the composition

1. `DemoBundle` declares its stable name and dependency on `FrameworkBundle`.
2. `DemoExtension` registers a greeting service and console commands in the DI container.
3. `DemoBundle` implements `RouteLoader` and contributes routes.
4. The app boots the kernel, resolves services, dispatches `DemoReady`, and shuts down.

Read the [demo source](https://github.com/Interchouette-ITC/Serenade/tree/dev/examples/demo-app)
alongside the [bundle guide](../../concepts/bundle-extensions/). Then continue to
[web modes](../choose-your-mode/).

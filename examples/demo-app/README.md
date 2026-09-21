# Demo Serenade app

Canonical **copy-paste** sample for a Serenade bundle: package config, `Extension`,
DI service, `RouteLoader` routes, console command `demo:hello`, and an event
subscriber on `demo.ready`.

Author guide: [`docs-dev/BUNDLES.md`](../../docs-dev/BUNDLES.md).

```bash
cargo run -p serenade-demo-app
```

Console entry (`bin/console` analogue):

```bash
cargo run -p serenade-demo-app --bin console
cargo run -p serenade-demo-app --bin console -- serenade:about
cargo run -p serenade-demo-app --bin console -- demo:hello
cargo run -p serenade-demo-app --bin console -- demo:slug "Hello World"
cargo run -p serenade-demo-app --bin console -- demo:workflow
cargo run -p serenade-demo-app --bin console -- debug:container --plain
```

## Wave 29–34 dogfood

| Crate | How this sample uses it |
| --- | --- |
| `serenade-string` | Console `demo:slug` (slug + pluralize) |
| `serenade-workflow` | Console `demo:workflow` (draft → published) |

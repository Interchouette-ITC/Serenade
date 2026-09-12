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
cargo run -p serenade-demo-app --bin console -- debug:container --plain
```

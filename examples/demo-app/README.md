# Demo Serenade app

Boots `FrameworkBundle` and `DemoBundle`, loads `config/packages/*.toml`, and prints the greeting service plus registered routes.

```bash
cargo run -p serenade-demo-app
```

Console entry (`bin/console` analogue):

```bash
cargo run -p serenade-demo-app --bin console
cargo run -p serenade-demo-app --bin console -- serenade:about
cargo run -p serenade-demo-app --bin console -- debug:container --plain
```

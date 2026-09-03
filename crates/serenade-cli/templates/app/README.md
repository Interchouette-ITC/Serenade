# __PACKAGE_NAME__

Scaffolded with `serenade new`. Dependencies are resolved by **Cargo** (not a second package manager).

```bash
cargo run
cargo run --bin console
cargo run --bin console -- serenade:about
```

Apply more package recipes from an app root:

```bash
cargo run -p serenade-cli --bin serenade -- recipe apply security --no-cargo
```

See Serenade `docs-dev/RECIPES.md`.

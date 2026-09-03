# Recipes and app scaffolding

Serenade reproduces the **Flex recipe** concept on top of **Cargo**. Cargo owns dependencies (`Cargo.toml`, `cargo add`, crates.io / git / path). The `serenade` CLI owns composition: skeleton trees and package config stubs.

## Commands

Build the CLI from this workspace:

```bash
cargo run -p serenade-cli --bin serenade -- --help
```

| Command | Role |
| --- | --- |
| `serenade new <name>` | Create an app skeleton (`config/packages/*.toml`, `.env.example`, `src/main.rs`, `src/bin/console.rs`) |
| `serenade recipe list` | List embedded recipes |
| `serenade recipe apply <id>` | Copy recipe files into an app root; run `cargo add` for declared crates unless `--no-cargo` |

Flags:

- `new --path DIR --force`
- `recipe apply --root DIR --force --no-cargo`

## Recipe format

Each recipe is a directory with `recipe.toml` plus optional `files/`:

```toml
id = "framework"
description = "Framework package config and registration hints"

[cargo]
dependencies = [
  { crate = "serenade-bundle", git = "https://github.com/Interchouette-ITC/Serenade.git", branch = "dev" },
]

[[files]]
src = "files/config/packages/framework.toml"
dest = "config/packages/framework.toml"

[hints]
bundles = ["FrameworkBundle"]
note = "Register FrameworkBundle and load FrameworkExtension."
```

Shipped recipes: `framework`, `security` (config stub until SecurityBundle lands).

## Package config

Default format is **TOML** under `config/packages/`. YAML remains loadable through `serenade-config`; recipes ship TOML.

## Console vs CLI

| Surface | Crate | Role |
| --- | --- | --- |
| Scaffolding | `serenade-cli` (`serenade` binary) | `new` / `recipe` |
| In-app console | `serenade-console` | `bin/console` analogue (`serenade:about`, `debug:container`, …) |

Do not invent a second package manager.

# Recipes and app scaffolding

Serenade reproduces the **Flex recipe** concept on top of **Cargo**. Cargo owns dependencies (`Cargo.toml`, `cargo add`, crates.io / git / path). The `serenade` CLI owns composition: skeleton trees and package config stubs.

## CLI stack

| Crate | Role |
| --- | --- |
| **clap** | Command tree and argument parsing |
| **cling** | Handler dispatch on top of clap (no giant `match`) |
| **clap_complete** | Shell completions (`serenade completion <shell>`) |
| **clap_mangen** | Man page (`serenade man [--output PATH]`) |

clap stays the parser. cling structures implementation. Completions and man pages are generated from the same `Command` tree (`CommandFactory` + `debug_assert` in tests).

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
| `serenade completion <shell>` | Print shell completions |
| `serenade man` | Print a man page (or `--output PATH`) |

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
| Scaffolding | `serenade-cli` (`serenade` binary) | `new` / `recipe` / completions / man |
| In-app console | `serenade-console` | `bin/console` analogue (`serenade:about`, `debug:container`, …) |

Do not invent a second package manager. In-app console still uses clap (+ optional ratatui for interactive debug); scaffolding uses clap + cling.

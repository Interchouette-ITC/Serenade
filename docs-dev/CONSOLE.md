# Console

Serenade’s `bin/console` analogue lives in `serenade-console`.

## Concepts

| Symfony | Serenade |
| --- | --- |
| `php bin/console …` | App binary (for example `cargo run -p serenade-demo-app --bin console`) |
| Console component | `serenade-console` + `Command` trait |
| Tagged `console.command` | DI tag `console.command` + `RegisterCommandsPass` |
| Flex scaffolding | [`RECIPES.md`](RECIPES.md) (`serenade` CLI); Cargo stays the package manager |

## Global flags

- `--env` / `APP_ENV` — runtime environment (`dev`, `test`, `prod`, or custom)
- `--no-debug` — force debug off even in `dev` / `test`

With no command (or `list`), the application prints registered commands.

## Built-in commands (FrameworkExtension)

| Command | Role |
| --- | --- |
| `serenade:about` | Version, environment, debug flag |
| `debug:container` | List DI service ids; **ratatui** TUI when stdout is a TTY (`--plain` for text) |

## Wiring

`build_container` always installs `RegisterCommandsPass`. `FrameworkExtension` registers the built-in commands. Apps resolve `console.application` and call `Application::run_with`.

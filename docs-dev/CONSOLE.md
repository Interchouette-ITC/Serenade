# Console

Serenade’s `bin/console` analogue lives in `serenade-console`.

## Concepts

| Symfony                  | Serenade                                                                     |
| ------------------------ | ---------------------------------------------------------------------------- |
| `php bin/console ...`      | App binary (for example `make console` / `make console ARGS='...'`)          |
| Console component        | `serenade-console` + `Command` trait                                         |
| Tagged `console.command` | DI tag `console.command` + `RegisterCommandsPass`                            |
| Flex scaffolding         | [`RECIPES.md`](RECIPES.md) (`serenade` CLI); Cargo stays the package manager |

## Global flags

- `--env` / `APP_ENV` - runtime environment (`dev`, `test`, `prod`, or custom)
- `--no-debug` - force debug off even in `dev` / `test`
- `--interactive` / `-i` - REPL with ↑/↓ history (`rustyline`, file `~/.serenade_history`)

With no command (or `list`), the application prints registered commands. Interactive mode lists commands once, then prompts `serenade>` until `quit` / `exit`.

## Built-in commands (FrameworkExtension)

| Command           | Role                                                                           |
| ----------------- | ------------------------------------------------------------------------------ |
| `serenade:about`  | Version, environment, debug flag                                               |
| `debug:container` | List DI service ids; **ratatui** TUI when stdout is a TTY (`--plain` for text) |
| `debug:config`    | Dump flattened parameters; **debug mode required**; secrets redacted unless `--reveal` |

### `debug:config` security

- Refuses to run when debug is off (`--no-debug`, or prod/custom without debug).
- Redacts keys whose names look like secrets (`password`, `secret`, `token`, `dsn`, …).
- `--reveal` shows raw values and still requires debug mode.
- Optional key prefix filter: `debug:config framework` (non-flag args).
- `--plain` forces text output (otherwise ratatui when stdout is a TTY).

## Wiring

`build_container` always installs `RegisterCommandsPass`. `FrameworkExtension` registers the built-in commands. Apps resolve `console.application` and call `Application::run_with`.

Call `serenade_config::load_dotenv(project_root, env)` before `build_container` when the app uses `.env` files. Pass the same environment name into `build_container` so `config/packages/{env}/` overlays apply.

Example:

```bash
make console ARGS='debug:config --plain'
make console ARGS='--interactive'
```

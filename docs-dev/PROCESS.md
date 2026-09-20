# Process

Child process helpers live in **`serenade-process`** ([#243](https://github.com/Interchouette-ITC/Serenade/issues/243)).

Symfony Process shaped: spawn, capture stdout/stderr, optional timeout, exit status.

## API

| Piece | Role |
| --- | --- |
| `Process::new` / `arg` / `args` | Command + arguments |
| `cwd` / `env` / `clear_env` | Working directory and environment |
| `timeout` | Kill child after duration |
| `run` | Capture output; non-zero exit is still `Ok` |
| `must_run` | Like `run`, but `ProcessError::Failed` on non-zero |
| `CompletedProcess` | `status` / `code` / `is_successful` / stdout / stderr |

## Example

```rust
use std::time::Duration;
use serenade_process::Process;

let out = Process::new("echo").arg("hi").must_run()?;
assert!(out.stdout_string().contains("hi"));

let err = Process::new("sleep")
    .arg("30")
    .timeout(Duration::from_millis(50))
    .run();
assert!(err.is_err());
```

## Limits

- Stdin is always null (no interactive streams)
- Timeout polls `try_wait` then `kill` (not a hard real-time scheduler)
- No PTY / TTY emulation
- Finder/glob is a separate crate in this wave

## Related

- [KERNEL.md](KERNEL.md) - component index
- [FILESYSTEM.md](FILESYSTEM.md) - path helpers

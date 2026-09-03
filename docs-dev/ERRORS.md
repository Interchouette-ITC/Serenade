# Errors

Serenade and apps follow modern Rust error habits (same family as typical `thiserror` SDKs):

| Layer | Habit |
| --- | --- |
| Library / domain | Typed `thiserror` enums; clear `Display` via `#[error(…)]`; `Debug` always |
| Wrapping | `#[from]` / `#[source]` / `#[error(transparent)]` to preserve chains |
| HTTP API (apps) | Map to status + JSON body with human `error` and stable `code` |
| Logs | Prefer `Debug` / sources; never put secrets in `Display` |

`Display` is for operators and clients. `Debug` is for developers. Serde is for wire formats, not a substitute for either.

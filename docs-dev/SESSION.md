# Session

Symfony HttpFoundation-shaped **session** support lives in **`serenade-session`**.

Flash bag and HTTP middleware are separate follow-ups. CSRF remains HMAC-stateless in `serenade-security` (no session required).

## Pieces

| Piece | Role |
| --- | --- |
| `Session` | In-request attribute bag (`get` / `set` / `remove` / `clear` / `invalidate`) |
| `SessionStore` | Persist attribute maps by opaque session id |
| `MemorySessionStore` | Process-local store (tests / single-node) |
| `CookieSession` | Load/save via store + session-id cookie |
| `CookieSessionOptions` | Cookie name, path, `HttpOnly`, `Secure`, `SameSite`, `Max-Age` |
| `DEFAULT_SESSION_COOKIE` | `SERENADE_SESSION` |

## Cookie + store habit

```rust
use std::sync::Arc;
use serenade_session::{CookieSession, MemorySessionStore, SessionStore};

let store = Arc::new(MemorySessionStore::new());
let cookies = CookieSession::new(store);

// Request: raw Cookie header from the adapter
let mut session = cookies.open(cookie_header)?;
session.set("user_id", "42");

// Response: optional Set-Cookie value
if let Some(set_cookie) = cookies.commit(&session)? {
    // response.with_header("set-cookie", set_cookie)
}
```

- New visitors get a fresh random id (32 bytes hex)
- Stale cookie ids that are missing from the store are **replaced** (avoids fixation on dead ids)
- `invalidate` deletes the store entry and emits `Max-Age=0`
- Attribute values are `String`; apps JSON-encode structured payloads when needed

## Relation to security

| Concern | Owner |
| --- | --- |
| CSRF tokens | `serenade-security` (`HmacCsrfTokenManager`) |
| Session stickiness / flash / login token storage | `serenade-session` (+ later middleware) |

## Non-goals (this slice)

- Flash bag API
- Kernel middleware that auto-loads sessions
- Redis / DB session cluster as a required v1 product
- Signed cookie that embeds the whole attribute map (id + store only for now)

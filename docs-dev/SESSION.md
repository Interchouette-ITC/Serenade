# Session

Symfony HttpFoundation-shaped **session** support lives in **`serenade-session`**.

CSRF remains HMAC-stateless in `serenade-security` (no session required).

## Pieces

| Piece                                          | Role                                                                               |
| ---------------------------------------------- | ---------------------------------------------------------------------------------- |
| `Session`                                      | In-request attribute bag (`get` / `set` / `remove` / `clear` / `invalidate`)       |
| `FlashBag`                                     | One-shot messages (`add` / `peek` / `get` / `all` / `clear`) via `session.flash()` |
| `SessionStore`                                 | Persist attribute maps by opaque session id                                        |
| `MemorySessionStore`                           | Process-local store (tests / single-node)                                          |
| `CookieSession`                                | Load/save via store + session-id cookie                                            |
| `CookieSessionOptions`                         | Cookie name, path, `HttpOnly`, `Secure`, `SameSite`, `Max-Age`                     |
| `SessionMiddleware` / `AsyncSessionMiddleware` | Open session on request, commit + `Set-Cookie` on response                         |
| `SESSION_ATTRIBUTE`                            | `_serenade_session` (request attribute key)                                        |
| `DEFAULT_SESSION_COOKIE`                       | `SERENADE_SESSION`                                                                 |
| `FLASH_SESSION_KEY`                            | `_serenade.flashes` (internal attribute)                                           |

## Kernel middleware habit

```rust
use std::sync::Arc;
use serenade_http::HttpKernel;
use serenade_session::{
    CookieSession, MemorySessionStore, SessionMiddleware, SessionStore, request_session_mut,
};

let store = Arc::new(MemorySessionStore::new());
let cookies = CookieSession::new(store);
let mut kernel = HttpKernel::new(|request| {
    let session = request_session_mut(request).expect("session middleware");
    session.set("user_id", "42");
    session.flash().add("success", "Saved");
    // ...
});
kernel.push_middleware(SessionMiddleware::new(cookies));
```

For `AsyncHttpKernel`, use `AsyncSessionMiddleware` the same way.

Helpers: `request_session` / `request_session_mut`.

## Cookie + store (without middleware)

```rust
use std::sync::Arc;
use serenade_session::{CookieSession, MemorySessionStore, SessionStore};

let store = Arc::new(MemorySessionStore::new());
let cookies = CookieSession::new(store);

let mut session = cookies.open(cookie_header)?;
session.set("user_id", "42");

if let Some(set_cookie) = cookies.commit(&session)? {
    // response.with_header("set-cookie", set_cookie)
}
```

- New visitors get a fresh random id (32 bytes hex)
- Stale cookie ids that are missing from the store are **replaced** (avoids fixation on dead ids)
- `invalidate` deletes the store entry and emits `Max-Age=0`
- Attribute values are `String`; apps JSON-encode structured payloads when needed
- Middleware commits only when the handler returns a `Response` (errors skip cookie/store write)

## Flash bag

```rust
session.flash().add("success", "Order placed");
let messages = session.flash().get("success"); // consumes
let peek = session.flash().peek("error");     // keeps
```

- `get` / `all` remove messages; `peek` / `peek_all` do not
- Empty kinds are ignored
- Messages survive `CookieSession::commit` / reload like any other session attribute

## Relation to security

| Concern                                          | Owner                                        |
| ------------------------------------------------ | -------------------------------------------- |
| CSRF tokens                                      | `serenade-security` (`HmacCsrfTokenManager`) |
| Session stickiness / flash / login token storage | `serenade-session` (`SessionMiddleware`)     |

See also [SECURITY.md](SECURITY.md).

## Non-goals

- Redis / DB session cluster as a required v1 product
- Signed cookie that embeds the whole attribute map (id + store only for now)

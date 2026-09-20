# Security

AuthN/Z hooks, CSRF tokens, and how HTML apps stay safe. OAuth 2.0 / OIDC **client**
helpers live behind Cargo feature `oauth`. LDAP **directory bind** helpers live
behind feature `ldap` (apps own the LDAP client; Serenade is not a directory server).

## Pieces

| Type                                                      | Role                                                                                     |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `UserInterface` / `InMemoryUser`                          | Principal id + roles                                                                     |
| `TokenInterface` / `UsernamePasswordToken`                | Authenticated flag, optional user, credentials echo                                      |
| `Voter` / `AccessDecisionManager`                         | Affirmative strategy (any `Grant` wins)                                                  |
| `Authenticator`                                           | App-owned credential check                                                               |
| `FirewallMiddleware`                                      | HTTP middleware: read header → authenticate → store token on request attributes          |
| `SessionMiddleware` / `AsyncSessionMiddleware`            | HTTP middleware: load/save session via `serenade-session` (see [SESSION.md](SESSION.md)) |
| `CsrfToken` / `CsrfTokenManager` / `HmacCsrfTokenManager` | Issue and validate CSRF tokens (stateless HMAC)                                          |
| `PasswordHasher` / `Argon2idPasswordHasher`               | Hash and verify passwords (Argon2id, PHC string)                                         |
| `login` / `logout` / `token_from_session`                 | Persist identity on `serenade-session` (id + roles only)                                 |
| `SessionTokenMiddleware` / `AsyncSessionTokenMiddleware`  | Restore session identity onto `_security_token`                                          |
| `OAuthClientConfig` / `build_authorization_request` (feature `oauth`) | PKCE authorize URL + state                                                               |
| `token_exchange_form` / `TokenExchanger` / `MockTokenExchanger` (feature `oauth`) | Token endpoint body + sync exchange trait                                      |
| `token_from_oidc_subject` / `subject_from_id_token` (feature `oauth`) | Map IdP subject → security token (JWT payload decode is **unverified**)        |
| `LdapBindConfig` / `LdapBinder` / `MockLdapBinder` (feature `ldap`) | Directory bind config + sync trait (apps own LDAP client)                        |
| `LdapAuthenticator` / `authenticate_ldap_password` (feature `ldap`) | `username:password` firewall authenticator + bind → token                        |
| `SECURITY_SESSION_KEY`                                    | `_serenade.security_token`                                                               |
| `CSRF_FIELD_NAME` (`_token`)                              | Default HTML field name (Symfony habit)                                                  |

Request attribute key: `_security_token` (`TOKEN_ATTRIBUTE`). Helper: `request_token(&request)`.

HTML forms wire CSRF through **`serenade-form`** (see [FORMS.md](FORMS.md)): forms enable CSRF by default and call the token manager on bind/render.

## Bearer / API key plug-in

Apps own authenticators. Example pattern for an admin API key or bearer token:

1. Implement `Authenticator::authenticate` (parse `Authorization: Bearer …` or a dedicated header).
2. On success, return `UsernamePasswordToken::authenticated(InMemoryUser::new(…), credentials)`.
3. Register `FirewallMiddleware::new("Authorization", authenticator)` on `HttpKernel` (first middleware is outermost).
4. Controllers read `request_token` and optionally run `AccessDecisionManager` + `RoleVoter` for subjects like `admin.area`.

Package config scaffold remains `config/packages/security.toml` from the `security` recipe (`enabled = false` until the app wires authenticators).

## CSRF (HMAC)

`HmacCsrfTokenManager::new(secret)` signs tokens as `nonce.mac` for a given intention id (usually the form name). Validation recomputes the MAC; no server-side session store is required for CSRF v0.

Use a long random app secret. Rotate only with a coordinated cutover (old tokens become invalid).

Session stickiness (HTML apps, flash, and login token storage via the session login bridge) is separate: register `SessionMiddleware` from `serenade-session` on the HTTP kernel ([SESSION.md](SESSION.md)). CSRF does not depend on that middleware.

## Password hashing

`Argon2idPasswordHasher` implements `PasswordHasher`:

```rust
use serenade_security::{Argon2idPasswordHasher, PasswordHasher};

let hasher = Argon2idPasswordHasher::new();
let hashed = hasher.hash("secret")?;
assert!(hasher.verify(&hashed, "secret")?);
assert!(!hasher.verify(&hashed, "wrong")?);
```

- Output is a PHC string (`$argon2id$…`)
- Empty plain passwords are rejected
- Malformed stored hashes return `SecurityError::Password`
- Apps own user rows and when to rehash after parameter changes

## Session login bridge

After a successful password (or other) check, store the identity on the session and restore it on later requests:

```rust
use serenade_security::{
    SessionTokenMiddleware, UsernamePasswordToken, InMemoryUser, login, logout,
};
use serenade_session::{SessionMiddleware, request_session_mut};

// Outer: session. Inner: restore `_security_token` from session when absent.
kernel.push_middleware(SessionMiddleware::new(cookies));
kernel.push_middleware(SessionTokenMiddleware::new());

// On login (controller):
login(
    request_session_mut(request).expect("session"),
    &UsernamePasswordToken::authenticated(InMemoryUser::new("alice", vec!["ROLE_USER".into()]), ""),
);

// On logout:
logout(request_session_mut(request).expect("session"));
```

- Only **user id + roles** are stored (never the password / credential echo)
- `SessionTokenMiddleware` does not overwrite a token already set (for example by `FirewallMiddleware`)
- CSRF stays HMAC-stateless and does not require this bridge

## XSS

Default HTML escaping for form render lives in **`serenade-form`** (`escape_html` / `escape_attr`). Controllers must not concatenate raw user input into HTML responses.

## OAuth 2.0 / OIDC client (feature `oauth`)

Enable with `serenade-security` feature `oauth`. Serenade shapes the **relying party**
handshake; apps own HTTP to the IdP and JWT signature verification.

```rust
use serenade_security::{
    OAuthClientConfig, MockTokenExchanger, TokenExchanger, build_authorization_request,
    parse_token_response, subject_from_id_token, token_exchange_form, token_from_oidc_subject,
    login,
};

// Example endpoints (replace with your IdP / Google / GitHub values):
let config = OAuthClientConfig::new(
    "client-id",
    "https://accounts.google.com/o/oauth2/v2/auth", // or GitHub authorize URL
    "https://oauth2.googleapis.com/token",
    "https://app.example/oauth/callback",
)
.with_scopes(["openid", "email", "profile"])
.with_client_secret("client-secret"); // omit for public PKCE clients

// 1) Login start: redirect the browser, store state + code_verifier in session.
let auth = build_authorization_request(&config)?;
// redirect to auth.url(); remember auth.state() and auth.code_verifier()

// 2) Callback: verify state, then POST token_exchange_form(...) to config.token_endpoint().
let body = token_exchange_form(&config, "authorization-code", "stored-verifier");
// let json = http_post(config.token_endpoint(), body)?;
// let tokens = parse_token_response(&json)?;

// Tests can skip HTTP:
let tokens = MockTokenExchanger::new(parse_token_response(
    r#"{"access_token":"at","id_token":"hdr.eyJzdWIiOiJ1MSJ9.sig"}"#,
)?)
.exchange_code(&config, "code", "verifier")?;

let subject = if let Some(id_token) = tokens.id_token.as_deref() {
    subject_from_id_token(id_token)? // unverified payload; verify JWKS in production
} else {
    "lookup-via-userinfo".to_owned()
};
let security_token = token_from_oidc_subject(subject, ["ROLE_USER"], tokens.access_token);
// login(session, &security_token);
```

| Provider | Authorize | Token |
| --- | --- | --- |
| Google (OIDC) | `https://accounts.google.com/o/oauth2/v2/auth` | `https://oauth2.googleapis.com/token` |
| GitHub (OAuth) | `https://github.com/login/oauth/authorize` | `https://github.com/login/oauth/access_token` |

GitHub returns JSON when you send `Accept: application/json` on the token POST. Prefer OIDC `openid` scope when the IdP supports it so `id_token` carries `sub`.

**Production:** verify ID tokens with the IdP JWKS before trusting `subject_from_id_token`. Store and compare `state`. Keep `code_verifier` server-side only.

## LDAP directory bind (feature `ldap`)

Enable with `serenade-security` feature `ldap`. Serenade shapes the **bind**
handshake; apps own the LDAP network client (for example `ldap3`).

```rust
use serenade_security::{
    LdapAuthenticator, LdapBindConfig, MockLdapBinder, LdapIdentity,
    authenticate_ldap_password, FirewallMiddleware, login,
};

let config = LdapBindConfig::new(
    "ldaps://ldap.example.com",
    "dc=example,dc=com",
    "uid={username},ou=people,dc=example,dc=com",
);
let _dn = config.user_dn("alice")?; // uid=alice,ou=people,...

// Production: implement LdapBinder with your LDAP client using config.uri() + user_dn().
// Tests:
let binder = MockLdapBinder::new(LdapIdentity::new(
    "uid=alice,ou=people,dc=example,dc=com",
    "alice",
    ["ROLE_USER"],
))
.with_credentials("alice", "secret");

let token = authenticate_ldap_password(&binder, "alice", "secret")?;
// login(session, &token);

// Or firewall header / basic-style credentials as username:password:
let firewall = FirewallMiddleware::new("X-Ldap-Credentials", LdapAuthenticator::new(binder));
```

| Piece | Role |
| --- | --- |
| `LdapBindConfig` | URI, base DN, `{username}` DN template |
| `LdapBinder` | Sync bind → `LdapIdentity` |
| `MockLdapBinder` | Tests without a directory |
| `LdapAuthenticator` | `Authenticator` for `username:password` credentials |

**Production:** use LDAPS or StartTLS; never log passwords; map directory groups to roles in your `LdapBinder` implementation.

## Non-goals

- Authorization server / IdP in Serenade
- Shipping an LDAP server or mandatory LDAP SDK
- Built-in user persistence
- Coupling CSRF to a server session (CSRF v0 stays HMAC-stateless; session is optional via `serenade-session`)

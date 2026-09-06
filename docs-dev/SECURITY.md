# Security

AuthN/Z hooks for HTTP and access checks. This is **not** a full OAuth/OIDC stack.

## Pieces

| Type | Role |
| --- | --- |
| `UserInterface` / `InMemoryUser` | Principal id + roles |
| `TokenInterface` / `UsernamePasswordToken` | Authenticated flag, optional user, credentials echo |
| `Voter` / `AccessDecisionManager` | Affirmative strategy (any `Grant` wins) |
| `Authenticator` | App-owned credential check |
| `FirewallMiddleware` | HTTP middleware: read header → authenticate → store token on request attributes |

Request attribute key: `_security_token` (`TOKEN_ATTRIBUTE`). Helper: `request_token(&request)`.

## Bearer / API key plug-in

Apps own authenticators. Example pattern for an admin API key or bearer token:

1. Implement `Authenticator::authenticate` (parse `Authorization: Bearer …` or a dedicated header).
2. On success, return `UsernamePasswordToken::authenticated(InMemoryUser::new(…), credentials)`.
3. Register `FirewallMiddleware::new("Authorization", authenticator)` on `HttpKernel` (first middleware is outermost).
4. Controllers read `request_token` and optionally run `AccessDecisionManager` + `RoleVoter` for subjects like `admin.area`.

Package config scaffold remains `config/packages/security.toml` from the `security` recipe (`enabled = false` until the app wires authenticators).

## Non-goals

- OAuth2 / OIDC providers
- Session cookies / CSRF
- Built-in user persistence

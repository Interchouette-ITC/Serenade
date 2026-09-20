# serenade-security

AuthN/Z hooks: users, tokens, voters, HTTP firewall middleware, CSRF tokens,
Argon2id password hashing, and session login bridge (`login` /
`SessionTokenMiddleware`).

Optional Cargo feature `oauth`: OAuth 2.0 / OIDC relying-party helpers (PKCE
authorization URL, token form body, `TokenExchanger`, subject →
`UsernamePasswordToken`). Not an authorization server. See
`docs-dev/SECURITY.md` and Forms (`serenade-form`).

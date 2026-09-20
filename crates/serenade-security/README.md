# serenade-security

AuthN/Z hooks: users, tokens, voters, HTTP firewall middleware, CSRF tokens,
Argon2id password hashing, and session login bridge (`login` /
`SessionTokenMiddleware`).

Optional Cargo features:

- `oauth`: OAuth 2.0 / OIDC relying-party helpers (PKCE, token form, subject map)
- `ldap`: LDAP directory bind helpers (`LdapBinder`, `LdapAuthenticator`, mock)

Not an authorization server or LDAP server. See `docs-dev/SECURITY.md` and Forms
(`serenade-form`).

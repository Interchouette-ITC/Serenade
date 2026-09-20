//! AuthN/Z hooks: users, tokens, voters, HTTP firewall, CSRF, password hashing,
//! and session login bridge.
//!
//! Optional feature `oauth` adds OAuth 2.0 / OIDC relying-party helpers (not an
//! authorization server). Optional feature `ldap` adds directory bind helpers
//! (apps own the LDAP client). Apps plug bearer or API-key authenticators into
//! [`FirewallMiddleware`]. CSRF uses [`HmacCsrfTokenManager`] (stateless HMAC).
//! Password hashing uses [`Argon2idPasswordHasher`]. Session stickiness for HTML
//! logins uses [`login`] / [`SessionTokenMiddleware`]. See `docs-dev/SECURITY.md`.

mod access;
mod csrf;
mod error;
mod firewall;
#[cfg(feature = "ldap")]
mod ldap;
#[cfg(feature = "oauth")]
mod oauth;
mod password;
mod session_bridge;
mod user;

pub use access::{AccessDecisionManager, RoleVoter, Subject, Vote, Voter};
pub use csrf::{CSRF_FIELD_NAME, CsrfToken, CsrfTokenManager, HmacCsrfTokenManager};
pub use error::SecurityError;
pub use firewall::{Authenticator, FirewallMiddleware, TOKEN_ATTRIBUTE, request_token};
#[cfg(feature = "ldap")]
pub use ldap::{
    LDAP_USERNAME_PLACEHOLDER, LdapAuthenticator, LdapBindConfig, LdapBinder, LdapIdentity,
    MockLdapBinder, authenticate_ldap_password, split_user_password, token_from_ldap_identity,
};
#[cfg(feature = "oauth")]
pub use oauth::{
    AuthorizationRequest, MockTokenExchanger, OAuthClientConfig, OAuthTokenResponse,
    TokenExchanger, build_authorization_request, build_authorization_request_with,
    decode_jwt_payload_json, parse_token_response, subject_from_id_token, token_exchange_form,
    token_from_oidc_subject,
};
pub use password::{Argon2idPasswordHasher, PasswordHasher};
pub use session_bridge::{
    AsyncSessionTokenMiddleware, SECURITY_SESSION_KEY, SessionTokenMiddleware, login, logout,
    token_from_session,
};
pub use user::{InMemoryUser, TokenInterface, UserInterface, UsernamePasswordToken};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

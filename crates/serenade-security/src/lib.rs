//! AuthN/Z hooks: users, tokens, voters, and HTTP firewall middleware.
//!
//! Full OAuth/OIDC is out of scope. Apps plug bearer or API-key authenticators
//! into [`FirewallMiddleware`]. See `docs-dev/SECURITY.md`.

mod access;
mod error;
mod firewall;
mod user;

pub use access::{AccessDecisionManager, RoleVoter, Subject, Vote, Voter};
pub use error::SecurityError;
pub use firewall::{Authenticator, FirewallMiddleware, TOKEN_ATTRIBUTE, request_token};
pub use user::{InMemoryUser, TokenInterface, UserInterface, UsernamePasswordToken};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

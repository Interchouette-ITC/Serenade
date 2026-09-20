//! LDAP directory bind helpers (feature `ldap`).
//!
//! Shapes config and a sync bind trait so apps can plug `ldap3` (or another
//! client) without Serenade owning the network stack. This is **not** an LDAP
//! server.

use std::sync::Arc;

use crate::firewall::Authenticator;
use crate::{InMemoryUser, SecurityError, UsernamePasswordToken};

/// Placeholder token replaced by [`LdapBindConfig::user_dn`].
pub const LDAP_USERNAME_PLACEHOLDER: &str = "{username}";

/// Connection / DN templates for an LDAP bind authenticator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LdapBindConfig {
    uri: String,
    base_dn: String,
    /// DN template with [`LDAP_USERNAME_PLACEHOLDER`] (for example `uid={username},ou=people,dc=example,dc=com`).
    user_dn_template: String,
}

impl LdapBindConfig {
    /// Creates bind settings. `user_dn_template` should contain `{username}`.
    #[must_use]
    pub fn new(
        uri: impl Into<String>,
        base_dn: impl Into<String>,
        user_dn_template: impl Into<String>,
    ) -> Self {
        Self {
            uri: uri.into(),
            base_dn: base_dn.into(),
            user_dn_template: user_dn_template.into(),
        }
    }

    /// LDAP URI (for example `ldaps://ldap.example.com:636`).
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Search / base DN.
    #[must_use]
    pub fn base_dn(&self) -> &str {
        &self.base_dn
    }

    /// User DN template containing [`LDAP_USERNAME_PLACEHOLDER`].
    #[must_use]
    pub fn user_dn_template(&self) -> &str {
        &self.user_dn_template
    }

    /// Expands [`LDAP_USERNAME_PLACEHOLDER`] in the DN template.
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::Ldap`] when `username` is empty.
    pub fn user_dn(&self, username: &str) -> Result<String, SecurityError> {
        if username.is_empty() {
            return Err(SecurityError::Ldap {
                message: "username is empty".to_owned(),
            });
        }
        Ok(self
            .user_dn_template
            .replace(LDAP_USERNAME_PLACEHOLDER, username))
    }
}

/// Identity returned after a successful directory bind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LdapIdentity {
    dn: String,
    username: String,
    roles: Vec<String>,
}

impl LdapIdentity {
    /// Creates an identity with directory DN, login name, and roles.
    #[must_use]
    pub fn new(
        dn: impl Into<String>,
        username: impl Into<String>,
        roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            dn: dn.into(),
            username: username.into(),
            roles: roles.into_iter().map(Into::into).collect(),
        }
    }

    /// Bound distinguished name.
    #[must_use]
    pub fn dn(&self) -> &str {
        &self.dn
    }

    /// Login name used for the bind.
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Roles granted after bind (app-owned mapping).
    #[must_use]
    pub fn roles(&self) -> &[String] {
        &self.roles
    }
}

/// Sync LDAP bind. Apps implement this with their LDAP client.
pub trait LdapBinder: Send + Sync {
    /// Binds as `username` / `password` and returns the directory identity.
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::Authentication`] or [`SecurityError::Ldap`] on failure.
    fn bind(&self, username: &str, password: &str) -> Result<LdapIdentity, SecurityError>;
}

/// Fixed-response binder for tests.
#[derive(Debug, Clone)]
pub struct MockLdapBinder {
    identity: LdapIdentity,
    expected_username: Option<String>,
    expected_password: Option<String>,
}

impl MockLdapBinder {
    /// Always returns `identity` (any non-empty password succeeds).
    #[must_use]
    pub const fn new(identity: LdapIdentity) -> Self {
        Self {
            identity,
            expected_username: None,
            expected_password: None,
        }
    }

    /// Requires exact username/password match before returning the identity.
    #[must_use]
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.expected_username = Some(username.into());
        self.expected_password = Some(password.into());
        self
    }
}

impl LdapBinder for MockLdapBinder {
    fn bind(&self, username: &str, password: &str) -> Result<LdapIdentity, SecurityError> {
        if password.is_empty() {
            return Err(SecurityError::Authentication {
                message: "empty password".to_owned(),
            });
        }
        if let (Some(expected_user), Some(expected_pass)) =
            (&self.expected_username, &self.expected_password)
        {
            if username != expected_user || password != expected_pass {
                return Err(SecurityError::Authentication {
                    message: "invalid ldap credentials".to_owned(),
                });
            }
        }
        Ok(self.identity.clone())
    }
}

/// Builds an authenticated security token from a successful LDAP identity.
#[must_use]
pub fn token_from_ldap_identity(
    identity: &LdapIdentity,
    password_echo: impl Into<String>,
) -> UsernamePasswordToken {
    UsernamePasswordToken::authenticated(
        InMemoryUser::new(identity.username(), identity.roles().to_vec()),
        password_echo,
    )
}

/// Binds then maps to a [`UsernamePasswordToken`].
///
/// # Errors
///
/// Propagates binder failures.
pub fn authenticate_ldap_password(
    binder: &dyn LdapBinder,
    username: &str,
    password: &str,
) -> Result<UsernamePasswordToken, SecurityError> {
    let identity = binder.bind(username, password)?;
    Ok(token_from_ldap_identity(&identity, password))
}

/// Firewall [`Authenticator`] that expects `username:password` in the credentials string.
#[derive(Clone)]
pub struct LdapAuthenticator {
    binder: Arc<dyn LdapBinder>,
}

impl LdapAuthenticator {
    /// Wraps a binder for header / form credentials shaped as `username:password`.
    #[must_use]
    pub fn new(binder: impl LdapBinder + 'static) -> Self {
        Self {
            binder: Arc::new(binder),
        }
    }
}

impl Authenticator for LdapAuthenticator {
    fn authenticate(
        &self,
        credentials: Option<&str>,
    ) -> Result<UsernamePasswordToken, SecurityError> {
        let Some(raw) = credentials else {
            return Err(SecurityError::Authentication {
                message: "missing credentials".to_owned(),
            });
        };
        let (username, password) = split_user_password(raw)?;
        authenticate_ldap_password(self.binder.as_ref(), username, password)
    }
}

/// Splits `username:password` (first colon). Password may contain colons.
///
/// # Errors
///
/// Returns [`SecurityError::Ldap`] when the shape is invalid.
pub fn split_user_password(raw: &str) -> Result<(&str, &str), SecurityError> {
    let Some((username, password)) = raw.split_once(':') else {
        return Err(SecurityError::Ldap {
            message: "credentials must be username:password".to_owned(),
        });
    };
    if username.is_empty() {
        return Err(SecurityError::Ldap {
            message: "username is empty".to_owned(),
        });
    }
    Ok((username, password))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::{TokenInterface, UserInterface};

    fn sample_config() -> LdapBindConfig {
        LdapBindConfig::new(
            "ldaps://ldap.example.com",
            "dc=example,dc=com",
            "uid={username},ou=people,dc=example,dc=com",
        )
    }

    fn sample_identity() -> LdapIdentity {
        LdapIdentity::new(
            "uid=alice,ou=people,dc=example,dc=com",
            "alice",
            ["ROLE_USER"],
        )
    }

    #[test]
    fn user_dn_expands_username() {
        let config = sample_config();
        assert_eq!(config.uri(), "ldaps://ldap.example.com");
        assert_eq!(config.base_dn(), "dc=example,dc=com");
        assert_eq!(
            config.user_dn_template(),
            "uid={username},ou=people,dc=example,dc=com"
        );
        assert_eq!(
            config.user_dn("alice").expect("dn"),
            "uid=alice,ou=people,dc=example,dc=com"
        );
    }

    #[test]
    fn user_dn_rejects_empty_username() {
        let err = sample_config().user_dn("").expect_err("empty");
        assert!(matches!(err, SecurityError::Ldap { .. }));
    }

    #[test]
    fn mock_bind_and_token() {
        let binder = MockLdapBinder::new(sample_identity()).with_credentials("alice", "secret");
        let token = authenticate_ldap_password(&binder, "alice", "secret").expect("ok");
        assert!(token.is_authenticated());
        assert_eq!(
            token.user().map(UserInterface::user_identifier),
            Some("alice")
        );
        assert_eq!(token.credentials(), Some("secret"));
    }

    #[test]
    fn mock_without_expected_credentials_accepts_any_user() {
        let binder = MockLdapBinder::new(sample_identity());
        let token = authenticate_ldap_password(&binder, "anyone", "secret").expect("ok");
        assert_eq!(
            token.user().map(UserInterface::user_identifier),
            Some("alice")
        );
    }

    #[test]
    fn mock_rejects_wrong_username() {
        let binder = MockLdapBinder::new(sample_identity()).with_credentials("alice", "secret");
        let err = authenticate_ldap_password(&binder, "eve", "secret").expect_err("bad");
        assert!(matches!(err, SecurityError::Authentication { .. }));
    }

    #[test]
    fn mock_rejects_wrong_password() {
        let binder = MockLdapBinder::new(sample_identity()).with_credentials("alice", "secret");
        let err = authenticate_ldap_password(&binder, "alice", "nope").expect_err("bad");
        assert!(matches!(err, SecurityError::Authentication { .. }));
    }

    #[test]
    fn mock_rejects_empty_password() {
        let binder = MockLdapBinder::new(sample_identity());
        let err = binder.bind("alice", "").expect_err("empty");
        assert!(matches!(err, SecurityError::Authentication { .. }));
    }

    #[test]
    fn authenticator_parses_colon_credentials() {
        let auth = LdapAuthenticator::new(
            MockLdapBinder::new(LdapIdentity::new(
                "uid=bob,ou=people,dc=example,dc=com",
                "bob",
                ["ROLE_USER"],
            ))
            .with_credentials("bob", "p:ass"),
        );
        let token = auth.authenticate(Some("bob:p:ass")).expect("ok");
        assert_eq!(
            token.user().map(UserInterface::user_identifier),
            Some("bob")
        );
    }

    #[test]
    fn authenticator_rejects_missing_and_bad_shape() {
        let auth = LdapAuthenticator::new(MockLdapBinder::new(sample_identity()));
        assert!(matches!(
            auth.authenticate(None),
            Err(SecurityError::Authentication { .. })
        ));
        assert!(matches!(
            auth.authenticate(Some("nocolon")),
            Err(SecurityError::Ldap { .. })
        ));
        assert!(matches!(
            auth.authenticate(Some(":nopass")),
            Err(SecurityError::Ldap { .. })
        ));
    }

    #[test]
    fn identity_accessors() {
        let id = sample_identity();
        assert_eq!(id.dn(), "uid=alice,ou=people,dc=example,dc=com");
        assert_eq!(id.username(), "alice");
        assert_eq!(id.roles(), &["ROLE_USER".to_owned()]);
        let token = token_from_ldap_identity(&id, "");
        assert!(token.is_authenticated());
    }

    #[test]
    fn split_user_password_ok() {
        assert_eq!(
            split_user_password("u:p:extra").expect("split"),
            ("u", "p:extra")
        );
    }
}

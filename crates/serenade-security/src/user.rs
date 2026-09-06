//! User and security token traits.

/// Authenticated (or anonymous) principal.
pub trait UserInterface: Send + Sync {
    /// Stable user identifier (login, UUID, …).
    fn user_identifier(&self) -> &str;

    /// Role names granted to this user.
    fn roles(&self) -> &[String];
}

/// Simple in-memory user for stubs and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryUser {
    id: String,
    roles: Vec<String>,
}

impl InMemoryUser {
    /// Creates a user with `id` and `roles`.
    #[must_use]
    pub fn new(id: impl Into<String>, roles: impl Into<Vec<String>>) -> Self {
        Self {
            id: id.into(),
            roles: roles.into(),
        }
    }
}

impl UserInterface for InMemoryUser {
    fn user_identifier(&self) -> &str {
        &self.id
    }

    fn roles(&self) -> &[String] {
        &self.roles
    }
}

/// Security token carrying credentials and optional user.
pub trait TokenInterface: Send + Sync {
    /// Whether authentication succeeded.
    fn is_authenticated(&self) -> bool;

    /// Authenticated user when present.
    fn user(&self) -> Option<&dyn UserInterface>;

    /// Raw credentials string when present (bearer, API key, …).
    fn credentials(&self) -> Option<&str>;
}

/// Username/password-style token stub (also used for bearer/API key strings).
#[derive(Debug, Clone)]
pub struct UsernamePasswordToken {
    user: Option<InMemoryUser>,
    credentials: Option<String>,
    authenticated: bool,
}

impl UsernamePasswordToken {
    /// Anonymous unauthenticated token.
    #[must_use]
    pub const fn anonymous() -> Self {
        Self {
            user: None,
            credentials: None,
            authenticated: false,
        }
    }

    /// Authenticated token for `user` with optional credential echo.
    #[must_use]
    pub fn authenticated(user: InMemoryUser, credentials: impl Into<String>) -> Self {
        Self {
            user: Some(user),
            credentials: Some(credentials.into()),
            authenticated: true,
        }
    }
}

impl TokenInterface for UsernamePasswordToken {
    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn user(&self) -> Option<&dyn UserInterface> {
        self.user.as_ref().map(|user| user as &dyn UserInterface)
    }

    fn credentials(&self) -> Option<&str> {
        self.credentials.as_deref()
    }
}

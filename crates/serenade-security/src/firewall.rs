//! HTTP firewall middleware (header → authenticator → request token attribute).

use std::sync::Arc;

use serenade_http::{HttpError, Middleware, Request, RequestHandler, Response};

use crate::{SecurityError, UsernamePasswordToken};

/// Request attribute key where the firewall stores the security token.
pub const TOKEN_ATTRIBUTE: &str = "_security_token";

/// Turns request credentials into a [`UsernamePasswordToken`].
pub trait Authenticator: Send + Sync {
    /// Authenticates using the raw header value (for example `Bearer …` or an API key).
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::Authentication`] when credentials are invalid.
    fn authenticate(
        &self,
        credentials: Option<&str>,
    ) -> Result<UsernamePasswordToken, SecurityError>;
}

/// Firewall that reads a header, runs an [`Authenticator`], and stores the token on the request.
pub struct FirewallMiddleware {
    header_name: String,
    authenticator: Arc<dyn Authenticator>,
    allow_anonymous: bool,
}

impl FirewallMiddleware {
    /// Creates a firewall reading `header_name` (default apps use `Authorization`).
    #[must_use]
    pub fn new(
        header_name: impl Into<String>,
        authenticator: impl Authenticator + 'static,
    ) -> Self {
        Self {
            header_name: header_name.into(),
            authenticator: Arc::new(authenticator),
            allow_anonymous: false,
        }
    }

    /// When `true`, missing credentials become an anonymous token instead of 401.
    #[must_use]
    pub const fn allow_anonymous(mut self, allow: bool) -> Self {
        self.allow_anonymous = allow;
        self
    }
}

impl Middleware for FirewallMiddleware {
    fn process(
        &self,
        request: &mut Request,
        next: &dyn RequestHandler,
    ) -> Result<Response, HttpError> {
        let credentials = request.headers().get(&self.header_name);
        let token = if credentials.is_none() && self.allow_anonymous {
            UsernamePasswordToken::anonymous()
        } else {
            self.authenticator
                .authenticate(credentials)
                .map_err(|err| HttpError::status(401, err.to_string()))?
        };
        request.attributes_mut().insert(TOKEN_ATTRIBUTE, token);
        next.handle(request)
    }
}

/// Returns the firewall token from request attributes when present.
#[must_use]
pub fn request_token(request: &Request) -> Option<&UsernamePasswordToken> {
    request
        .attributes()
        .get::<UsernamePasswordToken>(TOKEN_ATTRIBUTE)
}

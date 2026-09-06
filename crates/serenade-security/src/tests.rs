use serenade_http::{HttpKernel, Method, Request, Response};

use super::{
    request_token, version, AccessDecisionManager, Authenticator, FirewallMiddleware, InMemoryUser,
    RoleVoter, SecurityError, TokenInterface, UserInterface, UsernamePasswordToken, Vote, Voter,
    TOKEN_ATTRIBUTE,
};

struct ApiKeyAuthenticator {
    expected: &'static str,
}

impl Authenticator for ApiKeyAuthenticator {
    fn authenticate(
        &self,
        credentials: Option<&str>,
    ) -> Result<UsernamePasswordToken, SecurityError> {
        let Some(raw) = credentials else {
            return Err(SecurityError::Authentication {
                message: "missing credentials".to_owned(),
            });
        };
        let key = raw.strip_prefix("Bearer ").unwrap_or(raw);
        if key != self.expected {
            return Err(SecurityError::Authentication {
                message: "invalid api key".to_owned(),
            });
        }
        Ok(UsernamePasswordToken::authenticated(
            InMemoryUser::new("admin", vec!["ROLE_ADMIN".to_owned()]),
            key,
        ))
    }
}

struct DenyAll;

impl Voter for DenyAll {
    fn vote(&self, _token: &dyn TokenInterface, _subject: &str) -> Vote {
        Vote::Deny
    }
}

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn user_and_token_basics() {
    let user = InMemoryUser::new("u1", vec!["ROLE_USER".to_owned()]);
    assert_eq!(user.user_identifier(), "u1");
    assert_eq!(user.roles(), &["ROLE_USER".to_owned()]);

    let anon = UsernamePasswordToken::anonymous();
    assert!(!anon.is_authenticated());
    assert!(anon.user().is_none());

    let token = UsernamePasswordToken::authenticated(user, "secret");
    assert!(token.is_authenticated());
    assert_eq!(token.credentials(), Some("secret"));
    assert_eq!(token.user().expect("user").user_identifier(), "u1");
}

#[test]
fn access_decision_affirmative() {
    let mut adm = AccessDecisionManager::new();
    adm.add_voter(RoleVoter::new("ROLE_ADMIN", "admin.area"));
    let token = UsernamePasswordToken::authenticated(
        InMemoryUser::new("a", vec!["ROLE_ADMIN".to_owned()]),
        "k",
    );
    adm.decide(&token, "admin.area").expect("grant");
    let err = adm.decide(&token, "other").expect_err("no grant");
    assert!(matches!(err, SecurityError::AccessDenied { .. }));
}

#[test]
fn access_decision_deny_and_empty() {
    let mut adm = AccessDecisionManager::new();
    let err = adm
        .decide(&UsernamePasswordToken::anonymous(), "x")
        .expect_err("empty");
    assert!(matches!(err, SecurityError::AccessDenied { .. }));
    adm.add_voter(DenyAll);
    let err = adm
        .decide(&UsernamePasswordToken::anonymous(), "x")
        .expect_err("deny");
    assert!(matches!(err, SecurityError::AccessDenied { .. }));
}

#[test]
fn firewall_sets_token_attribute() {
    let mut kernel = HttpKernel::new(|request: &mut Request| {
        let token = request_token(request).expect("token");
        assert!(token.is_authenticated());
        assert_eq!(token.user().expect("user").user_identifier(), "admin");
        assert!(request.attributes().contains(TOKEN_ATTRIBUTE));
        Ok(Response::text(200, "ok"))
    });
    kernel.push_middleware(FirewallMiddleware::new(
        "Authorization",
        ApiKeyAuthenticator { expected: "secret" },
    ));
    let response = kernel
        .handle(Request::new(Method::Get, "/admin").with_header("Authorization", "Bearer secret"));
    assert_eq!(response.status(), 200);
}

#[test]
fn firewall_401_on_bad_credentials() {
    let mut kernel = HttpKernel::new(|_request: &mut Request| Ok(Response::text(200, "ok")));
    kernel.push_middleware(FirewallMiddleware::new(
        "Authorization",
        ApiKeyAuthenticator { expected: "secret" },
    ));
    let response = kernel
        .handle(Request::new(Method::Get, "/admin").with_header("Authorization", "Bearer wrong"));
    assert_eq!(response.status(), 401);
}

#[test]
fn firewall_anonymous_when_allowed() {
    let mut kernel = HttpKernel::new(|request: &mut Request| {
        let token = request_token(request).expect("token");
        assert!(!token.is_authenticated());
        Ok(Response::text(200, "anon"))
    });
    kernel.push_middleware(
        FirewallMiddleware::new("Authorization", ApiKeyAuthenticator { expected: "secret" })
            .allow_anonymous(true),
    );
    let response = kernel.handle(Request::new(Method::Get, "/"));
    assert_eq!(response.status(), 200);
}

#[test]
fn role_voter_denies_anonymous_and_wrong_role() {
    let voter = RoleVoter::new("ROLE_ADMIN", "admin.area");
    assert_eq!(
        voter.vote(&UsernamePasswordToken::anonymous(), "admin.area"),
        Vote::Deny
    );
    let user_token = UsernamePasswordToken::authenticated(
        InMemoryUser::new("u", vec!["ROLE_USER".to_owned()]),
        "k",
    );
    assert_eq!(voter.vote(&user_token, "admin.area"), Vote::Deny);
    assert_eq!(voter.vote(&user_token, "other"), Vote::Abstain);
}

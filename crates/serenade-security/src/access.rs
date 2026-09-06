//! Voter pattern and access decision manager.

use crate::{SecurityError, TokenInterface};

/// Affirmative / deny / abstain vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    /// Grant access.
    Grant,
    /// Deny access.
    Deny,
    /// No opinion.
    Abstain,
}

/// Subject of an access check (attribute string in v0).
pub type Subject = str;

/// Decides access for a token against a subject.
pub trait Voter: Send + Sync {
    /// Votes for `token` on `subject`.
    fn vote(&self, token: &dyn TokenInterface, subject: &Subject) -> Vote;
}

/// Collects voters and applies an affirmative strategy (any grant wins; any deny without grant fails).
#[derive(Default)]
pub struct AccessDecisionManager {
    voters: Vec<Box<dyn Voter>>,
}

impl AccessDecisionManager {
    /// Empty manager (always denies when no grant).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a voter.
    pub fn add_voter(&mut self, voter: impl Voter + 'static) {
        self.voters.push(Box::new(voter));
    }

    /// Returns `Ok(())` when access is granted.
    ///
    /// # Errors
    ///
    /// Returns [`SecurityError::AccessDenied`] when no voter grants (or a deny stands alone).
    pub fn decide(
        &self,
        token: &dyn TokenInterface,
        subject: &Subject,
    ) -> Result<(), SecurityError> {
        let mut granted = false;
        let mut denied = false;
        for voter in &self.voters {
            match voter.vote(token, subject) {
                Vote::Grant => granted = true,
                Vote::Deny => denied = true,
                Vote::Abstain => {}
            }
        }
        if granted {
            return Ok(());
        }
        if denied || !self.voters.is_empty() {
            return Err(SecurityError::AccessDenied {
                message: format!("subject `{subject}` denied"),
            });
        }
        Err(SecurityError::AccessDenied {
            message: "no voters registered".to_owned(),
        })
    }
}

/// Grants when the token is authenticated and holds `role`.
#[derive(Debug, Clone)]
pub struct RoleVoter {
    role: String,
    subject: String,
}

impl RoleVoter {
    /// Creates a voter that grants `subject` when the user has `role`.
    #[must_use]
    pub fn new(role: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            subject: subject.into(),
        }
    }
}

impl Voter for RoleVoter {
    fn vote(&self, token: &dyn TokenInterface, subject: &Subject) -> Vote {
        if subject != self.subject {
            return Vote::Abstain;
        }
        let Some(user) = token.user() else {
            return Vote::Deny;
        };
        if user.roles().iter().any(|role| role == &self.role) {
            Vote::Grant
        } else {
            Vote::Deny
        }
    }
}

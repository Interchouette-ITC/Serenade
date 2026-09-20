//! Transition guards.

use crate::error::WorkflowError;
use crate::marking::Marking;
use crate::transition::Transition;

/// Context passed to a guard before a transition applies.
#[derive(Debug)]
pub struct TransitionContext<'a> {
    /// Workflow name.
    pub workflow: &'a str,
    /// Subject id in the marking store.
    pub subject_id: &'a str,
    /// Transition about to run.
    pub transition: &'a Transition,
    /// Marking before the transition.
    pub marking: &'a Marking,
}

/// Decides whether a transition may proceed.
pub trait Guard: Send + Sync {
    /// Return `Ok(())` to allow, or [`WorkflowError::GuardBlocked`] to deny.
    ///
    /// # Errors
    ///
    /// Any [`WorkflowError`]; callers treat non-guard errors as hard failures.
    fn decide(&self, ctx: &TransitionContext<'_>) -> Result<(), WorkflowError>;
}

impl<F> Guard for F
where
    F: Fn(&TransitionContext<'_>) -> Result<(), WorkflowError> + Send + Sync,
{
    fn decide(&self, ctx: &TransitionContext<'_>) -> Result<(), WorkflowError> {
        self(ctx)
    }
}

/// Block a transition with a reason string.
#[must_use]
pub fn block(transition: &str, reason: impl Into<String>) -> WorkflowError {
    WorkflowError::GuardBlocked {
        transition: transition.to_owned(),
        reason: reason.into(),
    }
}

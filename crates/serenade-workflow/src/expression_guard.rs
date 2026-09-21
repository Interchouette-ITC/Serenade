//! [`Guard`] that evaluates a [`serenade_expression`] boolean formula.

use serenade_expression::{ExpressionContext, Value, evaluate_bool};

use crate::error::WorkflowError;
use crate::guard::{Guard, TransitionContext, block};

/// Blocks a transition when the expression evaluates to false.
///
/// Context variables:
/// - `subject_id`, `workflow`, `transition` (strings)
/// - `place.<name>` = `true` for each place in the current marking
///
/// # Examples
///
/// ```
/// use serenade_workflow::ExpressionGuard;
///
/// let guard = ExpressionGuard::new(r#"place.draft && !(subject_id == "guest")"#);
/// assert_eq!(
///     guard.expression(),
///     r#"place.draft && !(subject_id == "guest")"#
/// );
/// ```
#[derive(Debug, Clone)]
pub struct ExpressionGuard {
    expression: String,
}

impl ExpressionGuard {
    /// Creates a guard for `expression` (must evaluate to a truthy value to allow).
    #[must_use]
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
        }
    }

    /// Source expression string.
    #[must_use]
    pub fn expression(&self) -> &str {
        &self.expression
    }
}

impl Guard for ExpressionGuard {
    fn decide(&self, ctx: &TransitionContext<'_>) -> Result<(), WorkflowError> {
        let vars = transition_context_vars(ctx);
        match evaluate_bool(&self.expression, &vars) {
            Ok(true) => Ok(()),
            Ok(false) => Err(block(
                ctx.transition.name(),
                format!("expression rejected: {}", self.expression),
            )),
            Err(err) => Err(block(
                ctx.transition.name(),
                format!("expression error: {err}"),
            )),
        }
    }
}

fn transition_context_vars(ctx: &TransitionContext<'_>) -> ExpressionContext {
    let mut vars = ExpressionContext::new();
    vars.insert("subject_id", Value::string(ctx.subject_id));
    vars.insert("workflow", Value::string(ctx.workflow));
    vars.insert("transition", Value::string(ctx.transition.name()));
    for place in ctx.marking.places() {
        vars.insert(format!("place.{place}"), Value::Bool(true));
    }
    vars
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{DefinitionBuilder, MemoryMarkingStore, Workflow};

    #[test]
    fn expression_guard_blocks_guest() {
        let definition = DefinitionBuilder::new()
            .places(["draft", "published"])
            .edge("publish", "draft", "published")
            .build()
            .expect("def");
        let store = Arc::new(MemoryMarkingStore::new());
        let mut workflow = Workflow::new("article", definition, store);
        let guard = ExpressionGuard::new(r#"!(subject_id == "guest") && place.draft"#);
        assert_eq!(
            guard.expression(),
            r#"!(subject_id == "guest") && place.draft"#
        );
        workflow.add_guard("publish", Arc::new(guard));
        assert!(!workflow.can("guest", "publish"));
        assert!(workflow.can("alice", "publish"));
        workflow.apply("alice", "publish").expect("apply");
    }

    #[test]
    fn expression_guard_parse_error_blocks() {
        let definition = DefinitionBuilder::new()
            .places(["draft", "published"])
            .edge("publish", "draft", "published")
            .build()
            .expect("def");
        let store = Arc::new(MemoryMarkingStore::new());
        let mut workflow = Workflow::new("article", definition, store);
        workflow.add_guard("publish", Arc::new(ExpressionGuard::new("1 +")));
        let err = workflow.apply("x", "publish").expect_err("bad expr");
        assert!(matches!(err, WorkflowError::GuardBlocked { .. }));
    }
}

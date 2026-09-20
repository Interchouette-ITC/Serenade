//! Runtime workflow: can / apply / enabled transitions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::definition::Definition;
use crate::error::WorkflowError;
use crate::guard::{Guard, TransitionContext};
use crate::marking::Marking;
use crate::store::MarkingStore;
use crate::transition::Transition;

/// Named workflow bound to a [`MarkingStore`].
pub struct Workflow {
    name: String,
    definition: Definition,
    store: Arc<dyn MarkingStore>,
    /// Guards keyed by transition name (`*` = all transitions).
    guards: HashMap<String, Vec<Arc<dyn Guard>>>,
    listeners: Vec<Arc<dyn TransitionListener>>,
}

/// Hook notified after a successful `apply`.
pub trait TransitionListener: Send + Sync {
    /// Called once the new marking is stored.
    fn on_completed(&self, ctx: &CompletedContext<'_>);
}

/// Payload for [`TransitionListener::on_completed`].
#[derive(Debug)]
pub struct CompletedContext<'a> {
    /// Workflow name.
    pub workflow: &'a str,
    /// Subject id.
    pub subject_id: &'a str,
    /// Applied transition.
    pub transition: &'a Transition,
    /// Marking after apply.
    pub marking: &'a Marking,
}

impl Workflow {
    /// Bind a definition to a store.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        definition: Definition,
        store: Arc<dyn MarkingStore>,
    ) -> Self {
        Self {
            name: name.into(),
            definition,
            store,
            guards: HashMap::new(),
            listeners: Vec::new(),
        }
    }

    /// Workflow name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Underlying definition.
    #[must_use]
    pub const fn definition(&self) -> &Definition {
        &self.definition
    }

    /// Register a guard for one transition name, or `"*"` for every transition.
    pub fn add_guard(&mut self, transition: impl Into<String>, guard: Arc<dyn Guard>) {
        self.guards
            .entry(transition.into())
            .or_default()
            .push(guard);
    }

    /// Register a completion listener.
    pub fn add_listener(&mut self, listener: Arc<dyn TransitionListener>) {
        self.listeners.push(listener);
    }

    /// Effective marking for `subject_id` (stored or definition initial).
    #[must_use]
    pub fn marking(&self, subject_id: &str) -> Marking {
        self.store
            .get(subject_id)
            .unwrap_or_else(|| self.definition.initial().clone())
    }

    /// Whether `transition` is enabled and guards pass.
    #[must_use]
    pub fn can(&self, subject_id: &str, transition: &str) -> bool {
        self.try_can(subject_id, transition).is_ok()
    }

    /// Enabled transition names for the current marking (guards included).
    #[must_use]
    pub fn enabled_transitions(&self, subject_id: &str) -> Vec<&str> {
        self.definition
            .transitions()
            .iter()
            .filter(|t| self.can(subject_id, t.name()))
            .map(Transition::name)
            .collect()
    }

    /// Apply a transition and persist the new marking.
    ///
    /// # Errors
    ///
    /// Unknown / disabled transitions, or a blocking guard.
    pub fn apply(&self, subject_id: &str, transition: &str) -> Result<Marking, WorkflowError> {
        let t = self.definition.transition(transition).ok_or_else(|| {
            WorkflowError::UnknownTransition {
                transition: transition.to_owned(),
            }
        })?;
        let current = self.marking(subject_id);
        Self::ensure_enabled(t, &current)?;
        self.run_guards(subject_id, t, &current)?;

        let mut next = current;
        for place in t.from() {
            next.unmark(place);
        }
        for place in t.to() {
            next.mark(place.clone());
        }
        self.store.set(subject_id, next.clone());

        let completed = CompletedContext {
            workflow: &self.name,
            subject_id,
            transition: t,
            marking: &next,
        };
        for listener in &self.listeners {
            listener.on_completed(&completed);
        }
        Ok(next)
    }

    fn try_can(&self, subject_id: &str, transition: &str) -> Result<(), WorkflowError> {
        let t = self.definition.transition(transition).ok_or_else(|| {
            WorkflowError::UnknownTransition {
                transition: transition.to_owned(),
            }
        })?;
        let current = self.marking(subject_id);
        Self::ensure_enabled(t, &current)?;
        self.run_guards(subject_id, t, &current)
    }

    fn ensure_enabled(transition: &Transition, marking: &Marking) -> Result<(), WorkflowError> {
        let from: Vec<&str> = transition.from().iter().map(String::as_str).collect();
        if marking.has_all(from) {
            Ok(())
        } else {
            Err(WorkflowError::NotEnabled {
                transition: transition.name().to_owned(),
            })
        }
    }

    fn run_guards(
        &self,
        subject_id: &str,
        transition: &Transition,
        marking: &Marking,
    ) -> Result<(), WorkflowError> {
        let ctx = TransitionContext {
            workflow: &self.name,
            subject_id,
            transition,
            marking,
        };
        for key in ["*", transition.name()] {
            if let Some(guards) = self.guards.get(key) {
                for guard in guards {
                    guard.decide(&ctx)?;
                }
            }
        }
        Ok(())
    }
}

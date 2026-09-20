//! Workflow graph: places, transitions, initial marking.

use crate::error::WorkflowError;
use crate::marking::Marking;
use crate::transition::Transition;

/// Immutable places + transitions + initial marking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    places: Vec<String>,
    transitions: Vec<Transition>,
    initial: Marking,
}

impl Definition {
    /// Places in definition order.
    #[must_use]
    pub fn places(&self) -> &[String] {
        &self.places
    }

    /// Transitions in definition order.
    #[must_use]
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    /// Initial marking for subjects with no stored marking.
    #[must_use]
    pub const fn initial(&self) -> &Marking {
        &self.initial
    }

    /// Find a transition by name.
    #[must_use]
    pub fn transition(&self, name: &str) -> Option<&Transition> {
        self.transitions.iter().find(|t| t.name() == name)
    }
}

/// Builds a [`Definition`].
#[derive(Debug, Default)]
pub struct DefinitionBuilder {
    places: Vec<String>,
    transitions: Vec<Transition>,
    initial: Option<Marking>,
}

impl DefinitionBuilder {
    /// Empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a place (order preserved; duplicates rejected at build).
    #[must_use]
    pub fn place(mut self, name: impl Into<String>) -> Self {
        self.places.push(name.into());
        self
    }

    /// Add several places.
    #[must_use]
    pub fn places<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.places.extend(names.into_iter().map(Into::into));
        self
    }

    /// Add a transition.
    #[must_use]
    pub fn transition(mut self, transition: Transition) -> Self {
        self.transitions.push(transition);
        self
    }

    /// Convenience: single-from / single-to transition.
    #[must_use]
    pub fn edge(
        self,
        name: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Self {
        self.transition(Transition::new(name, [from], [to]))
    }

    /// Initial marking (defaults to the first place if omitted).
    #[must_use]
    pub fn initial(mut self, marking: Marking) -> Self {
        self.initial = Some(marking);
        self
    }

    /// Build and validate the definition.
    ///
    /// # Errors
    ///
    /// Returns [`WorkflowError`] when places/transitions are empty or inconsistent.
    pub fn build(self) -> Result<Definition, WorkflowError> {
        if self.places.is_empty() {
            return Err(WorkflowError::EmptyPlaces);
        }
        for place in &self.places {
            if place.is_empty() {
                return Err(WorkflowError::EmptyPlaceName);
            }
        }
        let mut seen_places = std::collections::BTreeSet::new();
        for place in &self.places {
            if !seen_places.insert(place.clone()) {
                return Err(WorkflowError::DuplicatePlace {
                    place: place.clone(),
                });
            }
        }

        let mut seen_transitions = std::collections::BTreeSet::new();
        for transition in &self.transitions {
            if transition.name().is_empty() {
                return Err(WorkflowError::EmptyTransitionName);
            }
            if !seen_transitions.insert(transition.name().to_owned()) {
                return Err(WorkflowError::DuplicateTransition {
                    transition: transition.name().to_owned(),
                });
            }
            if transition.from().is_empty() || transition.to().is_empty() {
                return Err(WorkflowError::EmptyPlaceName);
            }
            for place in transition.from().iter().chain(transition.to()) {
                if !seen_places.contains(place) {
                    return Err(WorkflowError::UnknownPlace {
                        place: place.clone(),
                        transition: transition.name().to_owned(),
                    });
                }
            }
        }

        let initial = match self.initial {
            Some(marking) => {
                for place in marking.places() {
                    if !seen_places.contains(place) {
                        return Err(WorkflowError::UnknownInitialPlace {
                            place: place.to_owned(),
                        });
                    }
                }
                marking
            }
            None => Marking::single(self.places[0].clone()),
        };

        Ok(Definition {
            places: self.places,
            transitions: self.transitions,
            initial,
        })
    }
}

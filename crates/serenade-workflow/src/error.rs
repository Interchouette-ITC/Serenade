//! Workflow errors.

/// Failure while building a definition or applying a transition.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    /// Definition has no places.
    #[error("workflow definition has no places")]
    EmptyPlaces,
    /// A place name is empty.
    #[error("place name must not be empty")]
    EmptyPlaceName,
    /// A transition name is empty.
    #[error("transition name must not be empty")]
    EmptyTransitionName,
    /// Transition references an unknown place.
    #[error("unknown place `{place}` on transition `{transition}`")]
    UnknownPlace {
        /// Place id.
        place: String,
        /// Transition name.
        transition: String,
    },
    /// Duplicate place in the definition.
    #[error("duplicate place `{place}`")]
    DuplicatePlace {
        /// Place id.
        place: String,
    },
    /// Duplicate transition name.
    #[error("duplicate transition `{transition}`")]
    DuplicateTransition {
        /// Transition name.
        transition: String,
    },
    /// Initial marking place is not in the definition.
    #[error("initial place `{place}` is not in the definition")]
    UnknownInitialPlace {
        /// Place id.
        place: String,
    },
    /// Transition is not enabled for the current marking.
    #[error("transition `{transition}` is not enabled")]
    NotEnabled {
        /// Transition name.
        transition: String,
    },
    /// Unknown transition name.
    #[error("unknown transition `{transition}`")]
    UnknownTransition {
        /// Transition name.
        transition: String,
    },
    /// A guard rejected the transition.
    #[error("guard blocked transition `{transition}`: {reason}")]
    GuardBlocked {
        /// Transition name.
        transition: String,
        /// Guard reason.
        reason: String,
    },
}

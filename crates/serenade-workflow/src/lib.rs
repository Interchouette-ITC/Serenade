//! State machine places, transitions, guards, and marking store (Symfony Workflow shaped).
//!
//! - [`Definition`] / [`DefinitionBuilder`] - places, transitions, initial marking
//! - [`Workflow`] - `can` / `apply` / `enabled_transitions`
//! - [`MarkingStore`] + [`MemoryMarkingStore`] - subject markings
//! - [`Guard`] - block illegal or policy-denied transitions
//!
//! # Examples
//!
//! ```
//! use std::sync::Arc;
//! use serenade_workflow::{
//!     DefinitionBuilder, MemoryMarkingStore, Transition, Workflow,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let definition = DefinitionBuilder::new()
//!     .places(["draft", "published", "archived"])
//!     .edge("publish", "draft", "published")
//!     .edge("archive", "published", "archived")
//!     .build()?;
//! let store = Arc::new(MemoryMarkingStore::new());
//! let workflow = Workflow::new("article", definition, store);
//! assert!(workflow.can("a1", "publish"));
//! workflow.apply("a1", "publish")?;
//! assert!(!workflow.can("a1", "publish"));
//! assert!(workflow.can("a1", "archive"));
//! # Ok(())
//! # }
//! ```

mod definition;
mod error;
mod guard;
mod marking;
mod memory;
mod store;
mod transition;
mod workflow;

pub use definition::{Definition, DefinitionBuilder};
pub use error::WorkflowError;
pub use guard::{Guard, TransitionContext, block};
pub use marking::Marking;
pub use memory::MemoryMarkingStore;
pub use store::MarkingStore;
pub use transition::Transition;
pub use workflow::{CompletedContext, TransitionListener, Workflow};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

//! Dependency injection container.
//!
//! Symfony-style service wiring without mandating a specific HTTP or ORM stack.

mod builder;
mod compile_pass;
mod container;
mod definition;
mod error;
mod parameter;
mod reference;

pub use builder::{ContainerBuilder, ServiceFactory};
pub use compile_pass::CompilePass;
pub use container::Container;
pub use definition::{Scope, ServiceDefinition};
pub use error::DiError;
pub use parameter::ParameterBag;
pub use reference::Reference;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

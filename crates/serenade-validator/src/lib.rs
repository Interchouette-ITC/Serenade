//! Constraint validation for DTOs and messenger commands.
//!
//! Built-in constraints (`NotBlank`, `Length`, `Range`) feed a [`RecursiveValidator`].
//! [`MessengerValidateHook`] plugs into `serenade-messenger` `ValidationMiddleware`.

mod constraint;
mod error;
mod messenger;
mod validator;
mod violation;

pub use constraint::{Constraint, Length, NotBlank, Range};
pub use error::ValidatorError;
pub use messenger::MessengerValidateHook;
pub use validator::{RecursiveValidator, Validatable, Validator};
pub use violation::{ConstraintViolationList, Violation};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

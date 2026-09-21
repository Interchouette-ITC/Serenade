//! Tiny safe rule expressions (Symfony `ExpressionLanguage` shaped).
//!
//! Boolean / arithmetic / property paths only. No function calls, no scripting VM.
//!
//! - [`evaluate`] / [`evaluate_bool`] - parse and run against [`ExpressionContext`]
//! - [`Value`] - bool, int, string
//! - Dotted paths resolve as flat context keys (`user.role`)

mod context;
mod error;
mod eval;
mod parse;
mod value;

pub use context::ExpressionContext;
pub use error::ExpressionError;
pub use eval::{evaluate, evaluate_bool};
pub use value::Value;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

//! Tiny safe rule expressions (Symfony `ExpressionLanguage` shaped).
//!
//! Boolean / arithmetic / property paths only. No function calls, no scripting VM.
//!
//! - [`evaluate`] / [`evaluate_bool`] - parse and run against [`ExpressionContext`]
//! - [`Value`] - bool, int, string
//! - Dotted paths resolve as flat context keys (`user.role`)
//!
//! # Examples
//!
//! ```
//! use serenade_expression::{ExpressionContext, Value, evaluate_bool};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut ctx = ExpressionContext::new();
//! ctx.insert("user.role", Value::string("admin"));
//! assert!(evaluate_bool(r#"user.role == "admin""#, &ctx)?);
//! # Ok(())
//! # }
//! ```

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

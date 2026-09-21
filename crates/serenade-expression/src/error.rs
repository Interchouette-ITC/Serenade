//! Expression evaluation errors.

/// Parse or evaluation failure.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ExpressionError {
    /// Lexer / parser rejected the source.
    #[error("expression parse error: {message}")]
    Parse {
        /// Human-readable reason.
        message: String,
    },
    /// Unknown variable or property path.
    #[error("unknown expression variable `{name}`")]
    UnknownVariable {
        /// Missing path.
        name: String,
    },
    /// Operator used on incompatible types.
    #[error("expression type error: {message}")]
    Type {
        /// Human-readable reason.
        message: String,
    },
    /// Integer division by zero.
    #[error("division by zero")]
    DivisionByZero,
}

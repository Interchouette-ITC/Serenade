//! Runtime values.

/// Boolean, integer, or string value in an expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Boolean literal / comparison result.
    Bool(bool),
    /// Signed 64-bit integer.
    Int(i64),
    /// UTF-8 string.
    Str(String),
}

impl Value {
    /// String value from `&str` / `String`.
    #[must_use]
    pub fn string(value: impl Into<String>) -> Self {
        Self::Str(value.into())
    }

    /// Coerce to bool (`Bool` as-is; non-zero `Int`; non-empty `Str`).
    #[must_use]
    pub fn as_bool_truthy(&self) -> bool {
        match self {
            Self::Bool(v) => *v,
            Self::Int(v) => *v != 0,
            Self::Str(v) => !v.is_empty(),
        }
    }
}

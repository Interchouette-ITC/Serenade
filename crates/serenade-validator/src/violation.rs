//! Constraint violations.

/// One failed constraint on a property path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Property path (empty for object-level).
    pub property_path: String,
    /// Human-readable message.
    pub message: String,
    /// Constraint type name (for example `NotBlank`).
    pub code: &'static str,
}

impl Violation {
    /// Creates a violation for `property_path`.
    #[must_use]
    pub fn new(
        property_path: impl Into<String>,
        message: impl Into<String>,
        code: &'static str,
    ) -> Self {
        Self {
            property_path: property_path.into(),
            message: message.into(),
            code,
        }
    }
}

/// Ordered list of [`Violation`]s.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConstraintViolationList {
    items: Vec<Violation>,
}

impl ConstraintViolationList {
    /// Creates an empty list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a violation.
    pub fn add(&mut self, violation: Violation) {
        self.items.push(violation);
    }

    /// Returns `true` when no violations were recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Number of violations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Borrowed slice of violations.
    #[must_use]
    pub fn as_slice(&self) -> &[Violation] {
        &self.items
    }
}

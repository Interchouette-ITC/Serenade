//! Constraint trait and built-in constraints.

use crate::ConstraintViolationList;

/// A rule that may append violations for a value.
pub trait Constraint: Send + Sync {
    /// Stable constraint code (for example `NotBlank`).
    fn code(&self) -> &'static str;

    /// Validates `value` for `property_path`, appending to `violations` on failure.
    fn validate(&self, value: &str, property_path: &str, violations: &mut ConstraintViolationList);
}

/// Rejects empty or whitespace-only strings.
#[derive(Debug, Default, Clone, Copy)]
pub struct NotBlank;

impl Constraint for NotBlank {
    fn code(&self) -> &'static str {
        "NotBlank"
    }

    fn validate(&self, value: &str, property_path: &str, violations: &mut ConstraintViolationList) {
        if value.trim().is_empty() {
            violations.add(crate::Violation::new(
                property_path,
                "This value should not be blank.",
                self.code(),
            ));
        }
    }
}

/// Enforces a string length range (inclusive).
#[derive(Debug, Clone, Copy)]
pub struct Length {
    /// Minimum length (inclusive).
    pub min: usize,
    /// Maximum length (inclusive).
    pub max: usize,
}

impl Length {
    /// Creates a length constraint.
    #[must_use]
    pub const fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }
}

impl Constraint for Length {
    fn code(&self) -> &'static str {
        "Length"
    }

    fn validate(&self, value: &str, property_path: &str, violations: &mut ConstraintViolationList) {
        let len = value.chars().count();
        if len < self.min || len > self.max {
            violations.add(crate::Violation::new(
                property_path,
                format!(
                    "This value should have between {} and {} characters.",
                    self.min, self.max
                ),
                self.code(),
            ));
        }
    }
}

/// Enforces an inclusive numeric range for a parsed `i64`.
#[derive(Debug, Clone, Copy)]
pub struct Range {
    /// Minimum value (inclusive).
    pub min: i64,
    /// Maximum value (inclusive).
    pub max: i64,
}

impl Range {
    /// Creates a range constraint.
    #[must_use]
    pub const fn new(min: i64, max: i64) -> Self {
        Self { min, max }
    }
}

impl Constraint for Range {
    fn code(&self) -> &'static str {
        "Range"
    }

    fn validate(&self, value: &str, property_path: &str, violations: &mut ConstraintViolationList) {
        let Ok(parsed) = value.parse::<i64>() else {
            violations.add(crate::Violation::new(
                property_path,
                "This value should be a valid integer.",
                self.code(),
            ));
            return;
        };
        if parsed < self.min || parsed > self.max {
            violations.add(crate::Violation::new(
                property_path,
                format!(
                    "This value should be between {} and {}.",
                    self.min, self.max
                ),
                self.code(),
            ));
        }
    }
}

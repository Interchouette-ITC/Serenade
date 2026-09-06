//! Validator interface and recursive implementation.

use crate::{Constraint, ConstraintViolationList};

/// Runs constraints and returns a violation list.
pub trait Validator: Send + Sync {
    /// Validates `value` at `property_path` against `constraints`.
    fn validate_value(
        &self,
        value: &str,
        property_path: &str,
        constraints: &[&dyn Constraint],
    ) -> ConstraintViolationList;
}

/// Default validator that applies each constraint in order.
#[derive(Debug, Default, Clone, Copy)]
pub struct RecursiveValidator;

impl RecursiveValidator {
    /// Creates the default validator.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Validator for RecursiveValidator {
    fn validate_value(
        &self,
        value: &str,
        property_path: &str,
        constraints: &[&dyn Constraint],
    ) -> ConstraintViolationList {
        let mut violations = ConstraintViolationList::new();
        for constraint in constraints {
            constraint.validate(value, property_path, &mut violations);
        }
        violations
    }
}

/// Object that can validate itself through a [`Validator`].
pub trait Validatable {
    /// Collects violations for this value.
    fn validate(&self, validator: &dyn Validator) -> ConstraintViolationList;
}

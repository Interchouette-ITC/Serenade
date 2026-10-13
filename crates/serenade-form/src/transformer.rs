//! View ↔ model transformers for form fields.

use crate::FormError;
use crate::kind::ChoiceOptions;

/// Converts between the HTML view string and the model string stored on the field.
///
/// # Examples
///
/// ```
/// use serenade_form::{BoolToStringTransformer, DataTransformer};
///
/// let t = BoolToStringTransformer::default();
/// assert_eq!(t.reverse_transform("1").unwrap(), "1");
/// assert_eq!(t.reverse_transform("").unwrap(), "0");
/// assert_eq!(t.transform("1").unwrap(), "1");
/// ```
pub trait DataTransformer: Send + Sync {
    /// Model value → view string used in HTML.
    ///
    /// # Errors
    ///
    /// Returns [`FormError::Transform`] when the model value cannot be shown.
    fn transform(&self, model: &str) -> Result<String, FormError>;

    /// Submitted view string → model value stored on the field.
    ///
    /// # Errors
    ///
    /// Returns [`FormError::Transform`] when the view value is invalid.
    fn reverse_transform(&self, view: &str) -> Result<String, FormError>;
}

/// Maps truthy/falsy view strings to normalized `"1"` / `"0"` model values.
#[derive(Debug, Clone, Copy, Default)]
pub struct BoolToStringTransformer;

impl DataTransformer for BoolToStringTransformer {
    fn transform(&self, model: &str) -> Result<String, FormError> {
        Ok(if is_truthy(model) {
            String::from("1")
        } else {
            String::new()
        })
    }

    fn reverse_transform(&self, view: &str) -> Result<String, FormError> {
        Ok(if is_truthy(view) {
            String::from("1")
        } else {
            String::from("0")
        })
    }
}

/// Parses and normalizes signed 64-bit integers as decimal strings.
#[derive(Debug, Clone, Copy, Default)]
pub struct I64ToStringTransformer;

impl DataTransformer for I64ToStringTransformer {
    fn transform(&self, model: &str) -> Result<String, FormError> {
        if model.is_empty() {
            return Ok(String::new());
        }
        model
            .parse::<i64>()
            .map(|n| n.to_string())
            .map_err(|_| FormError::Transform(String::from("invalid i64 model value")))
    }

    fn reverse_transform(&self, view: &str) -> Result<String, FormError> {
        if view.is_empty() {
            return Ok(String::new());
        }
        view.parse::<i64>()
            .map(|n| n.to_string())
            .map_err(|_| FormError::Transform(String::from("invalid i64 view value")))
    }
}

/// Ensures submitted values are members of a [`ChoiceOptions`] list.
#[derive(Debug, Clone)]
pub struct ChoiceToValueTransformer {
    options: ChoiceOptions,
}

impl ChoiceToValueTransformer {
    /// Builds a transformer from choice options.
    #[must_use]
    pub const fn new(options: ChoiceOptions) -> Self {
        Self { options }
    }
}

impl DataTransformer for ChoiceToValueTransformer {
    fn transform(&self, model: &str) -> Result<String, FormError> {
        if model.is_empty() {
            return Ok(String::new());
        }
        if self.options.multiple {
            let mut out = Vec::new();
            for part in split_multi(model) {
                if !self.is_allowed(part) {
                    return Err(FormError::Transform(format!(
                        "choice value {part:?} is not allowed"
                    )));
                }
                out.push(part.to_owned());
            }
            return Ok(join_multi(&out));
        }
        if self.is_allowed(model) {
            Ok(model.to_owned())
        } else {
            Err(FormError::Transform(format!(
                "choice value {model:?} is not allowed"
            )))
        }
    }

    fn reverse_transform(&self, view: &str) -> Result<String, FormError> {
        self.transform(view)
    }
}

impl ChoiceToValueTransformer {
    fn is_allowed(&self, value: &str) -> bool {
        self.options.choices.iter().any(|c| c.value == value)
    }
}

fn is_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "on" | "yes"
    )
}

/// Splits a multi-value model/view string on ASCII whitespace.
#[must_use]
pub fn split_multi(value: &str) -> Vec<&str> {
    value.split_whitespace().filter(|s| !s.is_empty()).collect()
}

/// Joins multi-values with a single space (stable round-trip for choice multi).
#[must_use]
pub fn join_multi(values: &[String]) -> String {
    values.join(" ")
}

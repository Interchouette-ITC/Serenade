//! Variable map for expression evaluation.

use std::collections::HashMap;

use crate::Value;

/// Flat key → [`Value`] map. Dotted paths are single keys (`user.role`).
#[derive(Debug, Clone, Default)]
pub struct ExpressionContext {
    vars: HashMap<String, Value>,
}

impl ExpressionContext {
    /// Empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace `key`.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        self.vars.insert(key.into(), value);
        self
    }

    /// Look up a path (exact key match).
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&Value> {
        self.vars.get(path)
    }
}

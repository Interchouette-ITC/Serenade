//! Twig-include analogue without a template engine.

/// Runs `body` and returns its HTML fragment (named nesting for builders).
#[must_use]
pub fn partial(body: impl FnOnce() -> String) -> String {
    body()
}

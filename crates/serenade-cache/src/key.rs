//! Shared logical cache key checks.

use crate::CacheError;

/// Rejects empty logical cache keys.
///
/// # Errors
///
/// Returns [`CacheError::InvalidKey`] when `key` is empty.
pub fn validate_logical_key(key: &str) -> Result<(), CacheError> {
    if key.is_empty() {
        return Err(CacheError::InvalidKey {
            key: key.to_owned(),
            message: "key must not be empty".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_logical_key;
    use crate::CacheError;

    #[test]
    fn empty_key_rejected() {
        assert!(matches!(
            validate_logical_key(""),
            Err(CacheError::InvalidKey { .. })
        ));
    }
}

//! Redis key prefix helpers.

use crate::CacheError;

/// Joins `prefix` and logical `key` into the Redis key.
///
/// # Errors
///
/// Returns [`CacheError::InvalidKey`] when `key` is empty.
pub fn redis_key(prefix: &str, key: &str) -> Result<String, CacheError> {
    validate_logical_key(key)?;
    Ok(format!("{prefix}{key}"))
}

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
    use super::{redis_key, validate_logical_key};
    use crate::CacheError;

    #[test]
    fn joins_prefix_and_key() {
        assert_eq!(
            redis_key("serenade:", "cart:1").expect("key"),
            "serenade:cart:1"
        );
    }

    #[test]
    fn empty_key_rejected() {
        assert!(matches!(
            validate_logical_key(""),
            Err(CacheError::InvalidKey { .. })
        ));
        assert!(matches!(
            redis_key("p:", ""),
            Err(CacheError::InvalidKey { .. })
        ));
    }
}

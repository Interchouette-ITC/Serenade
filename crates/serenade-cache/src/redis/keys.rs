//! Redis key prefix helpers.

use crate::CacheError;
use crate::key::validate_logical_key;

/// Joins `prefix` and logical `key` into the Redis key.
///
/// # Errors
///
/// Returns [`CacheError::InvalidKey`] when `key` is empty.
pub fn redis_key(prefix: &str, key: &str) -> Result<String, CacheError> {
    validate_logical_key(key)?;
    Ok(format!("{prefix}{key}"))
}

#[cfg(test)]
mod tests {
    use super::redis_key;
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
            redis_key("p:", ""),
            Err(CacheError::InvalidKey { .. })
        ));
    }
}

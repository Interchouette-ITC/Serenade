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

/// Redis SET of tag names attached to a logical item key.
#[must_use]
pub fn item_tags_key(prefix: &str, key: &str) -> String {
    format!("{prefix}\0tags\0{key}")
}

/// Redis SET of logical keys that carry `tag`.
#[must_use]
pub fn tag_members_key(prefix: &str, tag: &str) -> String {
    format!("{prefix}\0tag\0{tag}")
}

#[cfg(test)]
mod tests {
    use super::{item_tags_key, redis_key, tag_members_key};
    use crate::CacheError;

    #[test]
    fn joins_prefix_and_key() {
        assert_eq!(
            redis_key("serenade:", "cart:1").expect("key"),
            "serenade:cart:1"
        );
        assert_eq!(item_tags_key("p:", "k"), "p:\0tags\0k");
        assert_eq!(tag_members_key("p:", "t"), "p:\0tag\0t");
    }

    #[test]
    fn empty_key_rejected() {
        assert!(matches!(
            redis_key("p:", ""),
            Err(CacheError::InvalidKey { .. })
        ));
    }
}

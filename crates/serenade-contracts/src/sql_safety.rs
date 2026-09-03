//! Defense-in-depth checks for SQL bind / filter string parameters.
//!
//! Parameterized queries and query builders remain mandatory. This layer rejects
//! NUL and other C0 control bytes in values before they reach a driver.
//!
//! Checks are **on by default**. Disable only when deliberately accepting risk:
//! set [`SQL_SAFETY_DISABLE_ENV`] to `1` / `true` / `yes` / `on`, or use
//! [`SqlSafetyPolicy::disabled`].

use crate::PersistenceError;

/// Process environment variable that turns SQL parameter safety **off**.
///
/// Recognized disable values (ASCII case-insensitive for letters): `1`, `true`,
/// `yes`, `on`. Any other value (including unset) keeps safety **enabled**.
pub const SQL_SAFETY_DISABLE_ENV: &str = "SERENADE_DISABLE_SQL_SAFETY";

/// Whether SQL parameter safety is active for this process.
///
/// Reads [`SQL_SAFETY_DISABLE_ENV`]. Prefer [`SqlSafetyPolicy`] in tests so
/// parallel suites do not depend on mutating the environment.
#[must_use]
pub fn sql_safety_enabled() -> bool {
    std::env::var(SQL_SAFETY_DISABLE_ENV).map_or(true, |value| !is_disable_flag(&value))
}

fn is_disable_flag(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Policy for [`reject_unsafe_sql_param`] / [`SqlSafetyPolicy::reject_param`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SqlSafetyPolicy {
    enabled: bool,
}

impl Default for SqlSafetyPolicy {
    fn default() -> Self {
        Self::from_env()
    }
}

impl SqlSafetyPolicy {
    /// Policy from [`SQL_SAFETY_DISABLE_ENV`] (enabled unless explicitly disabled).
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            enabled: sql_safety_enabled(),
        }
    }

    /// Always run checks (ignore env).
    #[must_use]
    pub const fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Skip checks. Caller accepts the risk of unsafe parameter bytes.
    #[must_use]
    pub const fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Returns whether this policy runs checks.
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        self.enabled
    }

    /// Validates `value` when enabled; otherwise returns `value` unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`PersistenceError::InvalidInput`] when enabled and `value`
    /// contains NUL or a disallowed C0 control character.
    pub fn reject_param(self, value: &str) -> Result<&str, PersistenceError> {
        if !self.enabled {
            return Ok(value);
        }
        validate_param_bytes(value)?;
        Ok(value)
    }

    /// Owned variant of [`Self::reject_param`].
    ///
    /// # Errors
    ///
    /// Same as [`Self::reject_param`].
    pub fn reject_param_owned(self, value: String) -> Result<String, PersistenceError> {
        self.reject_param(&value)?;
        Ok(value)
    }
}

/// Validates `value` using [`SqlSafetyPolicy::from_env`].
///
/// # Errors
///
/// Returns [`PersistenceError::InvalidInput`] when safety is enabled and the
/// value fails validation.
pub fn reject_unsafe_sql_param(value: &str) -> Result<&str, PersistenceError> {
    SqlSafetyPolicy::from_env().reject_param(value)
}

/// Owned variant of [`reject_unsafe_sql_param`].
///
/// # Errors
///
/// Same as [`reject_unsafe_sql_param`].
pub fn reject_unsafe_sql_param_owned(value: String) -> Result<String, PersistenceError> {
    SqlSafetyPolicy::from_env().reject_param_owned(value)
}

fn validate_param_bytes(value: &str) -> Result<(), PersistenceError> {
    for (index, byte) in value.as_bytes().iter().copied().enumerate() {
        if byte == 0 {
            return Err(PersistenceError::InvalidInput {
                message: format!("SQL parameter contains NUL at byte index {index}"),
            });
        }
        if is_disallowed_c0_byte(byte) {
            return Err(PersistenceError::InvalidInput {
                message: format!(
                    "SQL parameter contains disallowed control byte 0x{byte:02X} at index {index}"
                ),
            });
        }
    }
    Ok(())
}

/// C0 controls other than tab / LF / CR.
const fn is_disallowed_c0_byte(byte: u8) -> bool {
    byte < 0x20 && byte != 0x09 && byte != 0x0A && byte != 0x0D
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_clean_values() {
        let policy = SqlSafetyPolicy::enabled();
        assert_eq!(policy.reject_param("").unwrap(), "");
        assert_eq!(
            policy
                .reject_param("550e8400-e29b-41d4-a716-446655440000")
                .unwrap(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(
            policy.reject_param("hello-world_1").unwrap(),
            "hello-world_1"
        );
        assert_eq!(policy.reject_param("line\nok\t").unwrap(), "line\nok\t");
    }

    #[test]
    fn rejects_nul_when_enabled() {
        let policy = SqlSafetyPolicy::enabled();
        let error = policy.reject_param("ab\0c").unwrap_err();
        assert!(matches!(error, PersistenceError::InvalidInput { .. }));
        assert!(error.to_string().contains("NUL"));
    }

    #[test]
    fn rejects_other_c0_when_enabled() {
        let policy = SqlSafetyPolicy::enabled();
        let error = policy.reject_param("x\u{0001}y").unwrap_err();
        assert!(matches!(error, PersistenceError::InvalidInput { .. }));
    }

    #[test]
    fn disabled_policy_skips_checks() {
        let policy = SqlSafetyPolicy::disabled();
        assert!(!policy.is_enabled());
        assert_eq!(policy.reject_param("ab\0c").unwrap(), "ab\0c");
        assert_eq!(
            policy.reject_param_owned("x\u{0001}y".into()).unwrap(),
            "x\u{0001}y"
        );
    }

    #[test]
    fn disable_flag_parser() {
        assert!(is_disable_flag("1"));
        assert!(is_disable_flag("TRUE"));
        assert!(is_disable_flag(" yes "));
        assert!(is_disable_flag("on"));
        assert!(!is_disable_flag("0"));
        assert!(!is_disable_flag("false"));
        assert!(!is_disable_flag(""));
    }
}

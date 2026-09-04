//! Persistence string-parameter hygiene at the Serenade boundary.
//!
//! This is **not** SQL-injection protection. Injection is prevented by
//! parameterized queries and query builders (application adapters). This module
//! only rejects NUL (`\0`) in string values so they cannot truncate or confuse
//! C APIs, drivers, logging, or other NUL-terminated interop layers.
//!
//! Checks are **on by default**. Disable only when deliberately accepting risk:
//! set [`PERSIST_PARAM_CHECK_DISABLE_ENV`] to `1` / `true` / `yes` / `on`, or use
//! [`PersistParamPolicy::disabled`].

use crate::PersistenceError;

/// Process environment variable that turns persist-param NUL checks **off**.
///
/// Recognized disable values (ASCII case-insensitive for letters): `1`, `true`,
/// `yes`, `on`. Any other value (including unset) keeps checks **enabled**.
pub const PERSIST_PARAM_CHECK_DISABLE_ENV: &str = "SERENADE_DISABLE_PERSIST_PARAM_CHECK";

/// Previous env name; still honored so existing opt-outs keep working.
const LEGACY_SQL_SAFETY_DISABLE_ENV: &str = "SERENADE_DISABLE_SQL_SAFETY";

/// Whether persist-param NUL checks are active for this process.
///
/// Prefer [`PersistParamPolicy`] in tests so parallel suites do not depend on
/// mutating the environment.
#[must_use]
pub fn persist_param_check_enabled() -> bool {
    !(env_disables(PERSIST_PARAM_CHECK_DISABLE_ENV) || env_disables(LEGACY_SQL_SAFETY_DISABLE_ENV))
}

fn env_disables(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| is_disable_flag(&value))
}

fn is_disable_flag(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Policy for [`reject_unsafe_sql_param`] / [`PersistParamPolicy::reject_param`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PersistParamPolicy {
    enabled: bool,
}

impl Default for PersistParamPolicy {
    fn default() -> Self {
        Self::from_env()
    }
}

impl PersistParamPolicy {
    /// Policy from env (enabled unless an explicit disable flag is set).
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            enabled: persist_param_check_enabled(),
        }
    }

    /// Always run checks (ignore env).
    #[must_use]
    pub const fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Skip checks. Caller accepts the risk of NUL in persistence strings.
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
    /// contains a NUL byte.
    pub fn reject_param(self, value: &str) -> Result<&str, PersistenceError> {
        if !self.enabled {
            return Ok(value);
        }
        reject_nul(value)?;
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

/// Rejects NUL in `value` using [`PersistParamPolicy::from_env`].
///
/// Tab, LF, CR, and other non-NUL bytes are allowed. This is input hygiene for
/// the persistence boundary, not an SQL-injection filter.
///
/// # Errors
///
/// Returns [`PersistenceError::InvalidInput`] when checks are enabled and
/// `value` contains `\0`.
pub fn reject_unsafe_sql_param(value: &str) -> Result<&str, PersistenceError> {
    PersistParamPolicy::from_env().reject_param(value)
}

/// Owned variant of [`reject_unsafe_sql_param`].
///
/// # Errors
///
/// Same as [`reject_unsafe_sql_param`].
pub fn reject_unsafe_sql_param_owned(value: String) -> Result<String, PersistenceError> {
    PersistParamPolicy::from_env().reject_param_owned(value)
}

fn reject_nul(value: &str) -> Result<(), PersistenceError> {
    if let Some(index) = value.as_bytes().iter().position(|&b| b == 0) {
        return Err(PersistenceError::InvalidInput {
            message: format!("NUL byte is not allowed in persistence parameter (index {index})"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_clean_and_whitespace_controls() {
        let policy = PersistParamPolicy::enabled();
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
        assert_eq!(policy.reject_param("line\nok\t\r").unwrap(), "line\nok\t\r");
        // Other C0 bytes (except NUL) are allowed: valid in many strings.
        assert_eq!(policy.reject_param("x\u{0001}y").unwrap(), "x\u{0001}y");
    }

    #[test]
    fn rejects_nul_when_enabled() {
        let policy = PersistParamPolicy::enabled();
        let error = policy.reject_param("ab\0c").unwrap_err();
        assert!(matches!(error, PersistenceError::InvalidInput { .. }));
        assert!(error.to_string().contains("NUL"));
    }

    #[test]
    fn disabled_policy_skips_checks() {
        let policy = PersistParamPolicy::disabled();
        assert!(!policy.is_enabled());
        assert_eq!(policy.reject_param("ab\0c").unwrap(), "ab\0c");
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

    #[test]
    fn reject_param_owned_and_free_fns() {
        let policy = PersistParamPolicy::enabled();
        assert_eq!(policy.reject_param_owned("ok".to_owned()).unwrap(), "ok");
        assert!(policy
            .reject_param_owned("a\0b".to_owned())
            .unwrap_err()
            .to_string()
            .contains("NUL"));
        assert_eq!(reject_unsafe_sql_param("clean").unwrap(), "clean");
        assert!(reject_unsafe_sql_param("x\0y").is_err());
        assert_eq!(reject_unsafe_sql_param_owned("z".to_owned()).unwrap(), "z");
        assert!(PersistParamPolicy::enabled().is_enabled());
        assert!(!PersistParamPolicy::disabled().is_enabled());
    }
}

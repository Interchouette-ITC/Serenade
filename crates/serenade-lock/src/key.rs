//! Lock resource validation and owner tokens.

use crate::LockError;

/// Resource name plus opaque owner token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockKey {
    resource: String,
    token: String,
}

impl LockKey {
    /// Builds a key after validating `resource`.
    ///
    /// # Errors
    ///
    /// Returns [`LockError::InvalidKey`] when `resource` is empty.
    pub fn new(resource: impl Into<String>, token: impl Into<String>) -> Result<Self, LockError> {
        let resource = resource.into();
        validate_resource(&resource)?;
        Ok(Self {
            resource,
            token: token.into(),
        })
    }

    /// Lock resource name.
    #[must_use]
    pub fn resource(&self) -> &str {
        &self.resource
    }

    /// Opaque owner token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Rejects empty resource names.
///
/// # Errors
///
/// Returns [`LockError::InvalidKey`] when `resource` is empty.
pub fn validate_resource(resource: &str) -> Result<(), LockError> {
    if resource.is_empty() {
        return Err(LockError::InvalidKey {
            key: resource.to_owned(),
            message: "resource must not be empty".to_owned(),
        });
    }
    Ok(())
}

/// Generates a random hex owner token.
///
/// # Errors
///
/// Returns [`LockError::Generation`] when the OS RNG fails.
pub fn generate_lock_token() -> Result<String, LockError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| LockError::Generation)?;
    Ok(bytes
        .iter()
        .fold(String::with_capacity(32), |mut out, byte| {
            use std::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
            out
        }))
}

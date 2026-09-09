//! View helper errors.

use thiserror::Error;

/// Error from view helpers such as [`crate::asset`].
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("{0}")]
pub struct ViewError(pub String);

impl ViewError {
    pub(crate) fn msg(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

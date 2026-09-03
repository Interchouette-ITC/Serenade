//! Runtime environment.

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::KernelError;

/// Runtime environment for a Serenade application.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    /// Local development.
    Dev,
    /// Automated tests.
    Test,
    /// Production.
    Prod,
}

impl Environment {
    /// Returns whether this environment enables debug by default.
    ///
    /// `Dev` and `Test` are debug; `Prod` is not.
    #[must_use]
    pub const fn is_debug(self) -> bool {
        matches!(self, Self::Dev | Self::Test)
    }

    /// Parses `dev`, `test`, or `prod` (ASCII case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::UnknownEnvironment`] when `name` is not one of those values.
    pub fn from_name(name: &str) -> Result<Self, KernelError> {
        match name.to_ascii_lowercase().as_str() {
            "dev" => Ok(Self::Dev),
            "test" => Ok(Self::Test),
            "prod" => Ok(Self::Prod),
            _ => Err(KernelError::UnknownEnvironment(name.to_owned())),
        }
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Dev => "dev",
            Self::Test => "test",
            Self::Prod => "prod",
        })
    }
}

impl FromStr for Environment {
    type Err = KernelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s)
    }
}

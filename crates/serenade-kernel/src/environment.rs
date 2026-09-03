//! Runtime environment.

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::KernelError;

/// Runtime environment for a Serenade application.
///
/// Well-known values are [`Self::Dev`], [`Self::Test`], and [`Self::Prod`].
/// Any other non-empty name becomes [`Self::Custom`] (for example `staging` or
/// `recette`), matching Symfony's free-form `APP_ENV` habit.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Environment {
    /// Local development.
    Dev,
    /// Automated tests.
    Test,
    /// Production.
    Prod,
    /// Application-defined environment (`staging`, `recette`, …).
    Custom(String),
}

impl Environment {
    /// Returns whether this environment enables debug by default.
    ///
    /// Only [`Self::Dev`] and [`Self::Test`] are debug. [`Self::Prod`] and
    /// [`Self::Custom`] are not; override with [`crate::Kernel::with_debug`].
    #[must_use]
    pub const fn is_debug(&self) -> bool {
        matches!(self, Self::Dev | Self::Test)
    }

    /// Parses an environment name (ASCII case-insensitive).
    ///
    /// `dev`, `test`, and `prod` map to the well-known variants. Any other
    /// non-empty name becomes [`Self::Custom`] with a lowercase value.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::UnknownEnvironment`] when `name` is empty after trim.
    pub fn from_name(name: &str) -> Result<Self, KernelError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(KernelError::UnknownEnvironment(name.to_owned()));
        }
        let lower = trimmed.to_ascii_lowercase();
        Ok(match lower.as_str() {
            "dev" => Self::Dev,
            "test" => Self::Test,
            "prod" => Self::Prod,
            _ => Self::Custom(lower),
        })
    }

    /// Stable lowercase name used on the wire and in logs.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Dev => "dev",
            Self::Test => "test",
            Self::Prod => "prod",
            Self::Custom(name) => name.as_str(),
        }
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Environment {
    type Err = KernelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s)
    }
}

impl From<Environment> for String {
    fn from(value: Environment) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for Environment {
    type Error = KernelError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_name(&value)
    }
}

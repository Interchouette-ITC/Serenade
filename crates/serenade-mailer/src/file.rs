//! File transport (writes message dumps under a directory).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::null::validate_for_send;
use crate::render::render_message;
use crate::{Email, MailerError, Transport};

/// Writes each message as a file under `directory` (Symfony `FileTransport`).
#[derive(Clone, Debug)]
pub struct FileTransport {
    directory: PathBuf,
}

impl FileTransport {
    /// Stores dumps under `directory` (created on first send when missing).
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    /// Target directory.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

impl Transport for FileTransport {
    fn send(&self, email: &Email) -> Result<(), MailerError> {
        validate_for_send(email)?;
        fs::create_dir_all(&self.directory).map_err(|error| MailerError::Io {
            message: error.to_string(),
        })?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path = self.directory.join(format!("{nanos}.message"));
        let body = render_message(email);
        fs::write(&path, body).map_err(|error| MailerError::Io {
            message: error.to_string(),
        })?;
        Ok(())
    }
}

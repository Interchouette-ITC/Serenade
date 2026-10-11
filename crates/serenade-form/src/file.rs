//! Uploaded file payload for [`crate::FieldKind::File`].

use std::path::Path;

use tempfile::NamedTempFile;

use crate::FormError;

/// How an uploaded file is retained after multipart parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileStorage {
    /// Keep the entire payload in memory ([`UploadedFile::as_bytes`]).
    #[default]
    Memory,
    /// Spill the payload to a named temporary file ([`UploadedFile::path`]).
    TempFile,
}

/// One uploaded file from a `multipart/form-data` part.
#[derive(Debug)]
pub struct UploadedFile {
    filename: Option<String>,
    content_type: Option<String>,
    payload: FilePayload,
}

#[derive(Debug)]
enum FilePayload {
    Bytes(Vec<u8>),
    Temp(NamedTempFile),
}

impl UploadedFile {
    /// Builds an in-memory upload (tests and Memory storage).
    #[must_use]
    pub const fn from_bytes(
        filename: Option<String>,
        content_type: Option<String>,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            filename,
            content_type,
            payload: FilePayload::Bytes(bytes),
        }
    }

    /// Builds a tempfile-backed upload.
    ///
    /// # Errors
    ///
    /// Returns [`FormError::Upload`] when the tempfile cannot be created or written.
    pub fn from_tempfile(
        filename: Option<String>,
        content_type: Option<String>,
        bytes: &[u8],
    ) -> Result<Self, FormError> {
        let mut file = NamedTempFile::new()
            .map_err(|err| FormError::Upload(format!("failed to create tempfile: {err}")))?;
        std::io::Write::write_all(&mut file, bytes)
            .map_err(|err| FormError::Upload(format!("failed to write tempfile: {err}")))?;
        Ok(Self {
            filename,
            content_type,
            payload: FilePayload::Temp(file),
        })
    }

    /// Client-supplied filename when present.
    #[must_use]
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Client-supplied `Content-Type` when present.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Byte length of the payload.
    #[must_use]
    pub fn len(&self) -> usize {
        match &self.payload {
            FilePayload::Bytes(bytes) => bytes.len(),
            FilePayload::Temp(file) => file
                .as_file()
                .metadata()
                .map_or(0, |meta| usize::try_from(meta.len()).unwrap_or(usize::MAX)),
        }
    }

    /// Returns `true` when the payload is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// In-memory bytes when storage is [`FileStorage::Memory`].
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.payload {
            FilePayload::Bytes(bytes) => Some(bytes.as_slice()),
            FilePayload::Temp(_) => None,
        }
    }

    /// Filesystem path when storage is [`FileStorage::TempFile`].
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match &self.payload {
            FilePayload::Bytes(_) => None,
            FilePayload::Temp(file) => Some(file.path()),
        }
    }

    /// Consumes the upload and returns the raw bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FormError::Upload`] when a tempfile cannot be read.
    pub fn into_bytes(self) -> Result<Vec<u8>, FormError> {
        match self.payload {
            FilePayload::Bytes(bytes) => Ok(bytes),
            FilePayload::Temp(file) => std::fs::read(file.path())
                .map_err(|err| FormError::Upload(format!("failed to read tempfile: {err}"))),
        }
    }
}

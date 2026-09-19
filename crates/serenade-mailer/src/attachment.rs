//! Attachment and inline (CID) parts for Mime.

/// How a binary part is presented in the message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentDisposition {
    /// Downloadable attachment (`Content-Disposition: attachment`).
    Attachment,
    /// Inline part, usually referenced from HTML via `cid:` (`Content-Disposition: inline`).
    Inline,
}

/// File or inline body part (filename + content type + raw bytes).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attachment {
    filename: String,
    content_type: String,
    body: Vec<u8>,
    disposition: ContentDisposition,
    content_id: Option<String>,
}

impl Attachment {
    /// Builds a downloadable attachment from raw bytes.
    #[must_use]
    pub fn from_bytes(
        filename: impl Into<String>,
        content_type: impl Into<String>,
        body: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            filename: filename.into(),
            content_type: content_type.into(),
            body: body.into(),
            disposition: ContentDisposition::Attachment,
            content_id: None,
        }
    }

    /// Builds an inline part with a Content-ID for HTML `cid:` references.
    ///
    /// `content_id` should be a token without angle brackets (for example `logo@serenade`).
    #[must_use]
    pub fn inline_from_bytes(
        filename: impl Into<String>,
        content_type: impl Into<String>,
        content_id: impl Into<String>,
        body: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            filename: filename.into(),
            content_type: content_type.into(),
            body: body.into(),
            disposition: ContentDisposition::Inline,
            content_id: Some(content_id.into()),
        }
    }

    /// Suggested filename.
    #[must_use]
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// MIME content type (for example `application/pdf`).
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Raw attachment bytes.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Attachment vs inline disposition.
    #[must_use]
    pub const fn disposition(&self) -> ContentDisposition {
        self.disposition
    }

    /// Content-ID without angle brackets when this part is inline.
    #[must_use]
    pub fn content_id(&self) -> Option<&str> {
        self.content_id.as_deref()
    }

    /// Whether this part is inline (CID).
    #[must_use]
    pub const fn is_inline(&self) -> bool {
        matches!(self.disposition, ContentDisposition::Inline)
    }
}

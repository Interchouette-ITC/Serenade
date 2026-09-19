//! Email message builder (Symfony `Email` analogue).

use crate::{Address, Attachment, Body, MailerError};

/// Outgoing email before a transport sends it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Email {
    from: Vec<Address>,
    to: Vec<Address>,
    cc: Vec<Address>,
    bcc: Vec<Address>,
    reply_to: Vec<Address>,
    subject: String,
    body: Body,
    attachments: Vec<Attachment>,
}

impl Email {
    /// Empty message.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a From address.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError`] when `address` is invalid.
    pub fn from(
        mut self,
        address: impl TryInto<Address, Error = MailerError>,
    ) -> Result<Self, MailerError> {
        self.from.push(address.try_into()?);
        Ok(self)
    }

    /// Adds a To address.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError`] when `address` is invalid.
    pub fn to(
        mut self,
        address: impl TryInto<Address, Error = MailerError>,
    ) -> Result<Self, MailerError> {
        self.to.push(address.try_into()?);
        Ok(self)
    }

    /// Adds a Cc address.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError`] when `address` is invalid.
    pub fn cc(
        mut self,
        address: impl TryInto<Address, Error = MailerError>,
    ) -> Result<Self, MailerError> {
        self.cc.push(address.try_into()?);
        Ok(self)
    }

    /// Adds a Bcc address.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError`] when `address` is invalid.
    pub fn bcc(
        mut self,
        address: impl TryInto<Address, Error = MailerError>,
    ) -> Result<Self, MailerError> {
        self.bcc.push(address.try_into()?);
        Ok(self)
    }

    /// Adds a Reply-To address.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError`] when `address` is invalid.
    pub fn reply_to(
        mut self,
        address: impl TryInto<Address, Error = MailerError>,
    ) -> Result<Self, MailerError> {
        self.reply_to.push(address.try_into()?);
        Ok(self)
    }

    /// Sets the subject.
    #[must_use]
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Sets a text-only body (clears HTML unless [`Self::html`] is used after via [`Self::body`]).
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        let html = self.body.html_part().map(str::to_owned);
        self.body = match html {
            Some(html) => Body::both(text, html),
            None => Body::text(text),
        };
        self
    }

    /// Sets an HTML-only body (keeps text when already set).
    #[must_use]
    pub fn html(mut self, html: impl Into<String>) -> Self {
        let text = self.body.text_part().map(str::to_owned);
        self.body = match text {
            Some(text) => Body::both(text, html),
            None => Body::html(html),
        };
        self
    }

    /// Replaces the body.
    #[must_use]
    pub fn body(mut self, body: Body) -> Self {
        self.body = body;
        self
    }

    /// Adds a downloadable attachment.
    #[must_use]
    pub fn attach(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Adds an inline (CID) part for HTML `cid:` references.
    #[must_use]
    pub fn embed(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Plain-text body when set.
    #[must_use]
    pub fn text_part(&self) -> Option<&str> {
        self.body.text_part()
    }

    /// HTML body when set.
    #[must_use]
    pub fn html_part(&self) -> Option<&str> {
        self.body.html_part()
    }

    /// Mime multipart tree for this message (see [`crate::MimeTree`]).
    #[must_use]
    pub fn mime_tree(&self) -> crate::MimeTree {
        crate::MimeTree::from_email(self)
    }

    /// From addresses.
    #[must_use]
    pub fn from_addresses(&self) -> &[Address] {
        &self.from
    }

    /// To addresses.
    #[must_use]
    pub fn to_addresses(&self) -> &[Address] {
        &self.to
    }

    /// Cc addresses.
    #[must_use]
    pub fn cc_addresses(&self) -> &[Address] {
        &self.cc
    }

    /// Bcc addresses.
    #[must_use]
    pub fn bcc_addresses(&self) -> &[Address] {
        &self.bcc
    }

    /// Reply-To addresses.
    #[must_use]
    pub fn reply_to_addresses(&self) -> &[Address] {
        &self.reply_to
    }

    /// Subject line.
    #[must_use]
    pub fn subject_line(&self) -> &str {
        &self.subject
    }

    /// Body parts.
    #[must_use]
    pub const fn message_body(&self) -> &Body {
        &self.body
    }

    /// Attachments.
    #[must_use]
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }
}

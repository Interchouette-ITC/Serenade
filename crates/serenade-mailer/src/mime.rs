//! Mime body tree: how Serenade structures multipart messages.

use crate::attachment::{Attachment, ContentDisposition};
use crate::email::Email;

/// Top-level Mime body layout for an [`Email`].
///
/// This is the framework view of multipart structure. SMTP encoding still goes
/// through lettre when the `smtp` feature is enabled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MimeTree {
    /// Single body part (text or HTML without nested multiparts).
    Single(MimePart),
    /// `multipart/alternative`: plain text plus an HTML (or related) sibling.
    Alternative {
        /// Plain-text sibling when present.
        text: Option<MimePart>,
        /// HTML sibling: either a single HTML part or `multipart/related`.
        html: Box<Self>,
    },
    /// `multipart/related` wrapping HTML plus inline CID parts.
    Related {
        /// Root content (usually a single HTML part).
        root: Box<Self>,
        /// Inline parts referenced via `cid:`.
        related: Vec<MimePart>,
    },
    /// `multipart/mixed` with a primary body and downloadable attachments.
    Mixed {
        /// Primary message body (single, alternative, or related).
        body: Box<Self>,
        /// Downloadable attachments.
        attachments: Vec<MimePart>,
    },
}

/// One leaf Mime part.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MimePart {
    /// MIME content type.
    pub content_type: String,
    /// Suggested filename when this is a file part.
    pub filename: Option<String>,
    /// Content disposition.
    pub disposition: ContentDisposition,
    /// Content-ID without angle brackets when inline.
    pub content_id: Option<String>,
    /// Decoded body bytes (UTF-8 for text parts).
    pub body: Vec<u8>,
}

impl MimePart {
    /// Text body part.
    #[must_use]
    pub fn text(body: &str) -> Self {
        Self {
            content_type: "text/plain; charset=utf-8".to_owned(),
            filename: None,
            disposition: ContentDisposition::Inline,
            content_id: None,
            body: body.as_bytes().to_vec(),
        }
    }

    /// HTML body part.
    #[must_use]
    pub fn html(body: &str) -> Self {
        Self {
            content_type: "text/html; charset=utf-8".to_owned(),
            filename: None,
            disposition: ContentDisposition::Inline,
            content_id: None,
            body: body.as_bytes().to_vec(),
        }
    }

    /// File or inline attachment part.
    #[must_use]
    pub fn from_attachment(attachment: &Attachment) -> Self {
        Self {
            content_type: attachment.content_type().to_owned(),
            filename: Some(attachment.filename().to_owned()),
            disposition: attachment.disposition(),
            content_id: attachment.content_id().map(str::to_owned),
            body: attachment.body().to_vec(),
        }
    }
}

impl MimeTree {
    /// Builds the Mime tree for an email (multipart rules used by SMTP encode).
    ///
    /// Nesting matches common client expectations:
    /// - text + HTML → `multipart/alternative`
    /// - HTML + CID inline → `multipart/related` (as the HTML alternative sibling when text exists)
    /// - downloadable files → outer `multipart/mixed`
    #[must_use]
    pub fn from_email(email: &Email) -> Self {
        let inline: Vec<MimePart> = email
            .attachments()
            .iter()
            .filter(|part| part.is_inline())
            .map(MimePart::from_attachment)
            .collect();
        let downloadable: Vec<MimePart> = email
            .attachments()
            .iter()
            .filter(|part| !part.is_inline())
            .map(MimePart::from_attachment)
            .collect();

        let body = content_tree(email, &inline);
        if downloadable.is_empty() {
            body
        } else {
            Self::Mixed {
                body: Box::new(body),
                attachments: downloadable,
            }
        }
    }

    /// Multipart subtype at this node (`alternative`, `related`, `mixed`), if any.
    #[must_use]
    pub const fn multipart_subtype(&self) -> Option<&'static str> {
        match self {
            Self::Single(_) => None,
            Self::Alternative { .. } => Some("alternative"),
            Self::Related { .. } => Some("related"),
            Self::Mixed { .. } => Some("mixed"),
        }
    }
}

fn content_tree(email: &Email, inline: &[MimePart]) -> MimeTree {
    let text = email.text_part().map(MimePart::text);
    let html = email.html_part().map(MimePart::html);
    let html_tree = html.map(|part| {
        let single = MimeTree::Single(part);
        if inline.is_empty() {
            single
        } else {
            MimeTree::Related {
                root: Box::new(single),
                related: inline.to_vec(),
            }
        }
    });

    match (text, html_tree) {
        (Some(text_part), Some(html_part)) => MimeTree::Alternative {
            text: Some(text_part),
            html: Box::new(html_part),
        },
        (Some(text_part), None) => {
            // Inline without HTML is unusual; keep CID parts under related around text.
            if inline.is_empty() {
                MimeTree::Single(text_part)
            } else {
                MimeTree::Related {
                    root: Box::new(MimeTree::Single(text_part)),
                    related: inline.to_vec(),
                }
            }
        }
        (None, Some(html_part)) => html_part,
        (None, None) => MimeTree::Single(MimePart::text("")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Attachment;
    use crate::Email;

    #[test]
    fn plain_only_is_single() {
        let email = Email::new()
            .from("a@b.test")
            .expect("from")
            .to("c@d.test")
            .expect("to")
            .subject("s")
            .text("hi");
        let tree = MimeTree::from_email(&email);
        assert!(matches!(tree, MimeTree::Single(_)));
        assert_eq!(tree.multipart_subtype(), None);
    }

    #[test]
    fn text_and_html_are_alternative() {
        let email = Email::new()
            .from("a@b.test")
            .expect("from")
            .to("c@d.test")
            .expect("to")
            .subject("s")
            .text("hi")
            .html("<p>hi</p>");
        let tree = MimeTree::from_email(&email);
        assert_eq!(tree.multipart_subtype(), Some("alternative"));
    }

    #[test]
    fn inline_and_file_nest_related_inside_mixed() {
        let email = Email::new()
            .from("a@b.test")
            .expect("from")
            .to("c@d.test")
            .expect("to")
            .subject("s")
            .html("<img src=\"cid:logo@serenade\" />")
            .embed(Attachment::inline_from_bytes(
                "logo.png",
                "image/png",
                "logo@serenade",
                vec![1, 2, 3],
            ))
            .attach(Attachment::from_bytes(
                "invoice.pdf",
                "application/pdf",
                vec![9],
            ));
        let MimeTree::Mixed { body, attachments } = MimeTree::from_email(&email) else {
            panic!("expected mixed");
        };
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].filename.as_deref(), Some("invoice.pdf"));
        let MimeTree::Related { root, related } = *body else {
            panic!("expected related");
        };
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].content_id.as_deref(), Some("logo@serenade"));
        assert!(matches!(*root, MimeTree::Single(_)));
    }

    #[test]
    fn text_html_inline_puts_related_under_alternative() {
        let email = Email::new()
            .from("a@b.test")
            .expect("from")
            .to("c@d.test")
            .expect("to")
            .subject("s")
            .text("hi")
            .html("<img src=\"cid:x\" />")
            .embed(Attachment::inline_from_bytes(
                "x.png",
                "image/png",
                "x",
                vec![1],
            ));
        let MimeTree::Alternative { text, html } = MimeTree::from_email(&email) else {
            panic!("expected alternative");
        };
        assert!(text.is_some());
        assert_eq!(html.multipart_subtype(), Some("related"));
    }
}

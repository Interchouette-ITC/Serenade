//! Render an [`Email`] as a readable message dump (file / debug).

use crate::Email;

/// Formats headers + body as a simple message dump (not a full MIME encoder).
#[must_use]
pub fn render_message(email: &Email) -> String {
    let mut out = String::new();
    append_addrs(&mut out, "From", email.from_addresses());
    append_addrs(&mut out, "To", email.to_addresses());
    append_addrs(&mut out, "Cc", email.cc_addresses());
    append_addrs(&mut out, "Bcc", email.bcc_addresses());
    append_addrs(&mut out, "Reply-To", email.reply_to_addresses());
    out.push_str("Subject: ");
    out.push_str(email.subject_line());
    out.push_str("\r\n");
    if let Some(subtype) = email.mime_tree().multipart_subtype() {
        out.push_str("X-Serenade-Mime: multipart/");
        out.push_str(subtype);
        out.push_str("\r\n");
    }
    out.push_str("\r\n");
    if let Some(text) = email.message_body().text_part() {
        out.push_str(text);
        out.push_str("\r\n");
    }
    if let Some(html) = email.message_body().html_part() {
        if email.message_body().text_part().is_some() {
            out.push_str("\r\n-- html --\r\n");
        }
        out.push_str(html);
        out.push_str("\r\n");
    }
    for attachment in email.attachments() {
        if attachment.is_inline() {
            out.push_str("\r\n-- inline: ");
            out.push_str(attachment.filename());
            out.push_str(" cid:");
            out.push_str(attachment.content_id().unwrap_or(""));
        } else {
            out.push_str("\r\n-- attachment: ");
            out.push_str(attachment.filename());
        }
        out.push_str(" (");
        out.push_str(attachment.content_type());
        out.push_str(", ");
        out.push_str(&attachment.body().len().to_string());
        out.push_str(" bytes) --\r\n");
    }
    out
}

fn append_addrs(out: &mut String, header: &str, addresses: &[crate::Address]) {
    if addresses.is_empty() {
        return;
    }
    out.push_str(header);
    out.push_str(": ");
    for (index, address) in addresses.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(&address.to_string());
    }
    out.push_str("\r\n");
}

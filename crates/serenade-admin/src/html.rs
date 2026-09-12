//! Minimal HTML for list and show pages.

use serenade_form::escape_html;

use crate::{AdminResource, AdminRow};

/// Renders a simple HTML table for list fields.
#[must_use]
pub fn render_list_html(resource: &AdminResource, rows: &[AdminRow]) -> String {
    let mut out = String::new();
    out.push_str("<section class=\"serenade-admin-list\">\n");
    out.push_str("<h1>");
    out.push_str(&escape_html(resource.name()));
    out.push_str("</h1>\n<table>\n<thead><tr>");
    for field in resource.list() {
        out.push_str("<th>");
        out.push_str(&escape_html(field.label()));
        out.push_str("</th>");
    }
    out.push_str("<th></th></tr></thead>\n<tbody>\n");
    for row in rows {
        out.push_str("<tr>");
        for field in resource.list() {
            out.push_str("<td>");
            out.push_str(&escape_html(row.get(field.name())));
            out.push_str("</td>");
        }
        let show_href = format!(
            "{}/{}",
            resource.list_path().trim_end_matches('/'),
            escape_html(row.id())
        );
        out.push_str("<td><a href=\"");
        out.push_str(&show_href);
        out.push_str("\">show</a></td></tr>\n");
    }
    out.push_str("</tbody>\n</table>\n</section>\n");
    out
}

/// Renders a simple definition list for show fields.
#[must_use]
pub fn render_show_html(resource: &AdminResource, row: &AdminRow) -> String {
    let mut out = String::new();
    out.push_str("<section class=\"serenade-admin-show\">\n");
    out.push_str("<h1>");
    out.push_str(&escape_html(resource.name()));
    out.push_str(" #");
    out.push_str(&escape_html(row.id()));
    out.push_str("</h1>\n<dl>\n");
    for field in resource.show() {
        out.push_str("<dt>");
        out.push_str(&escape_html(field.label()));
        out.push_str("</dt><dd>");
        out.push_str(&escape_html(row.get(field.name())));
        out.push_str("</dd>\n");
    }
    out.push_str("</dl>\n<p><a href=\"");
    out.push_str(&escape_html(&resource.list_path()));
    out.push_str("\">back to list</a></p>\n</section>\n");
    out
}

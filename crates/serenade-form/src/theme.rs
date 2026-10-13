//! Escaped HTML theme helpers (`form_row`, `form_widget`, `form_label`, `form_errors`).

use std::fmt::Write as _;

use serenade_validator::ConstraintViolationList;

use crate::form::Field;
use crate::kind::{ChoiceOptions, FieldConfig};
use crate::transformer::split_multi;
use crate::{Form, FormError, escape_attr, escape_html};

/// Renders a field label (`<label for="…">…</label>`), HTML-escaped.
///
/// Hidden fields render an empty string (no visible label).
#[must_use]
pub fn form_label(field: &Field) -> String {
    if matches!(field.config(), FieldConfig::Hidden(_)) {
        return String::new();
    }
    if matches!(
        field.config(),
        FieldConfig::Choice(ChoiceOptions { expanded: true, .. })
    ) {
        return String::new();
    }
    let id = escape_attr(field.name());
    let label = escape_html(field.name());
    format!(r#"<label for="{id}">{label}</label>"#)
}

/// Renders the field widget only (no label), with values and attributes escaped.
///
/// # Errors
///
/// Returns [`FormError::Transform`] when a field transformer rejects the model value.
pub fn form_widget(field: &Field) -> Result<String, FormError> {
    let id = escape_attr(field.name());
    let name = escape_attr(field.name());
    let view = view_value(field)?;
    let value = escape_attr(&view);
    let html = match field.config() {
        FieldConfig::Textarea(options) => {
            let rows = options
                .rows
                .map_or(String::new(), |r| format!(r#" rows="{r}""#));
            let placeholder = placeholder_attr(options.placeholder.as_deref());
            format!(
                r#"<textarea id="{id}" name="{name}"{rows}{placeholder}>{text}</textarea>"#,
                text = escape_html(&view),
            )
        }
        FieldConfig::Hidden(_) => {
            format!(r#"<input type="hidden" id="{id}" name="{name}" value="{value}" />"#)
        }
        FieldConfig::Password(options) => {
            let placeholder = placeholder_attr(options.placeholder.as_deref());
            format!(
                r#"<input type="password" id="{id}" name="{name}" value="{value}"{placeholder} />"#
            )
        }
        FieldConfig::Checkbox(options) => {
            let checked = if field.value() == options.checked_value
                || matches!(field.value(), "1" | "true")
            {
                " checked"
            } else {
                ""
            };
            let checked_value = escape_attr(&options.checked_value);
            format!(
                r#"<input type="checkbox" id="{id}" name="{name}" value="{checked_value}"{checked} />"#
            )
        }
        FieldConfig::Choice(options) => render_choice_widget(field, options, &id, &name),
        FieldConfig::Number(options) => {
            let min = options
                .min
                .map_or(String::new(), |m| format!(r#" min="{m}""#));
            let max = options
                .max
                .map_or(String::new(), |m| format!(r#" max="{m}""#));
            let step = options
                .step
                .map_or(String::new(), |s| format!(r#" step="{s}""#));
            format!(
                r#"<input type="number" id="{id}" name="{name}" value="{value}"{min}{max}{step} />"#
            )
        }
        FieldConfig::File(options) => {
            let accept = options.accept.as_deref().map_or(String::new(), |a| {
                format!(r#" accept="{}""#, escape_attr(a))
            });
            format!(r#"<input type="file" id="{id}" name="{name}"{accept} />"#)
        }
        FieldConfig::Text(options) => {
            let placeholder = placeholder_attr(options.placeholder.as_deref());
            format!(r#"<input type="text" id="{id}" name="{name}" value="{value}"{placeholder} />"#)
        }
    };
    Ok(html)
}

/// Renders escaped error messages for `field` from `violations`.
#[must_use]
pub fn form_errors(field: &Field, violations: &ConstraintViolationList) -> String {
    let messages: Vec<&str> = violations
        .as_slice()
        .iter()
        .filter(|v| v.property_path == field.name())
        .map(|v| v.message.as_str())
        .collect();
    if messages.is_empty() {
        return String::new();
    }
    let mut html = String::from(r#"<ul class="form-errors">"#);
    for message in messages {
        let _ = write!(html, "<li>{}</li>", escape_html(message));
    }
    html.push_str("</ul>");
    html
}

/// Renders label + widget + errors for one field (all escaped).
///
/// # Errors
///
/// Returns [`FormError::Transform`] when widget rendering fails.
pub fn form_row(field: &Field, violations: &ConstraintViolationList) -> Result<String, FormError> {
    let label = form_label(field);
    let widget = form_widget(field)?;
    let errors = form_errors(field, violations);
    Ok(format!(
        r#"<div class="form-row">{label}{widget}{errors}</div>"#
    ))
}

/// Renders the CSRF hidden input when CSRF is enabled on `form`.
///
/// # Errors
///
/// Returns [`FormError::Csrf`] when CSRF is enabled but no token was prepared or bound.
pub fn form_csrf(form: &Form) -> Result<String, FormError> {
    form.csrf_html()
}

fn view_value(field: &Field) -> Result<String, FormError> {
    field
        .transformer
        .as_ref()
        .map_or_else(|| Ok(field.value.clone()), |t| t.transform(&field.value))
}

fn placeholder_attr(placeholder: Option<&str>) -> String {
    placeholder.map_or(String::new(), |p| {
        format!(r#" placeholder="{}""#, escape_attr(p))
    })
}

fn render_choice_widget(field: &Field, options: &ChoiceOptions, id: &str, name: &str) -> String {
    let selected = split_multi(field.value());
    let legend = escape_html(field.name());
    if options.expanded {
        let mut html = format!(r"<fieldset><legend>{legend}</legend>");
        for (i, choice) in options.choices.iter().enumerate() {
            let input_id = format!("{id}_{i}");
            let checked = if selected.contains(&choice.value.as_str()) {
                " checked"
            } else {
                ""
            };
            let input_type = if options.multiple {
                "checkbox"
            } else {
                "radio"
            };
            let _ = write!(
                html,
                r#"<label for="{input_id}"><input type="{input_type}" id="{input_id}" name="{name}" value="{value}"{checked} />{choice_label}</label>"#,
                value = escape_attr(&choice.value),
                choice_label = escape_html(&choice.label),
            );
        }
        html.push_str("</fieldset>");
        return html;
    }
    let multiple = if options.multiple { " multiple" } else { "" };
    let mut html = format!(r#"<select id="{id}" name="{name}"{multiple}>"#);
    for choice in &options.choices {
        let is_selected = selected.contains(&choice.value.as_str());
        let selected_attr = if is_selected { " selected" } else { "" };
        let _ = write!(
            html,
            r#"<option value="{value}"{selected_attr}>{label}</option>"#,
            value = escape_attr(&choice.value),
            label = escape_html(&choice.label),
        );
    }
    html.push_str("</select>");
    html
}

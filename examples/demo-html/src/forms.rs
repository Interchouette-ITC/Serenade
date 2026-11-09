use std::sync::Arc;

use serenade_form::{
    ChoiceEntry, ChoiceOptions, FieldConfig, FileOptions, FileStorage, Form, HiddenOptions,
    TextareaOptions, form_csrf, form_widget,
};
use serenade_http::{HttpError, RouteCollection};
use serenade_security::HmacCsrfTokenManager;
use serenade_validator::NotBlank;
use serenade_view::{escape_attr, escape_html, path};

use crate::html::emoji_picker;
use crate::state::MAX_IMAGE_BYTES;
use crate::store::Post;

fn extract_csrf_hidden(form_html: &str) -> String {
    let marker = r#"<input type="hidden" name="_token""#;
    let Some(start) = form_html.find(marker) else {
        return String::new();
    };
    let rest = &form_html[start..];
    let Some(end) = rest.find("/>") else {
        return String::new();
    };
    rest[..=end + 1].to_owned()
}

pub(super) fn not_blank() -> Vec<Arc<dyn serenade_validator::Constraint>> {
    vec![Arc::new(NotBlank) as Arc<dyn serenade_validator::Constraint>]
}

pub(super) fn build_post_form(
    csrf: &HmacCsrfTokenManager,
    routes: &RouteCollection,
    categories: &[String],
) -> Result<String, HttpError> {
    let action = path(routes, "post_create", &[])?;
    let cancel = path(routes, "feed", &[])?;
    build_composer_form(csrf, categories, "post", &action, None, "Post", &cancel)
}

pub(super) fn build_edit_form(
    csrf: &HmacCsrfTokenManager,
    routes: &RouteCollection,
    categories: &[String],
    post: &Post,
) -> Result<String, HttpError> {
    let name = format!("edit-{}", post.id);
    let id = post.id.to_string();
    let action = path(routes, "admin_post_edit_post", &[("id", id.as_str())])?;
    let cancel = path(routes, "feed", &[])?;
    build_composer_form(
        csrf,
        categories,
        &name,
        &action,
        Some(post),
        "Save changes",
        &cancel,
    )
}

fn image_file_options() -> FileOptions {
    FileOptions {
        accept: Some("image/*".into()),
        max_size: MAX_IMAGE_BYTES as u64,
        mime_types: vec![
            "image/png".into(),
            "image/jpeg".into(),
            "image/jpg".into(),
            "image/webp".into(),
            "image/gif".into(),
        ],
        storage: FileStorage::Memory,
    }
}

fn category_choice_options(categories: &[String]) -> ChoiceOptions {
    let mut choices: Vec<ChoiceEntry> = categories.iter().map(ChoiceEntry::new).collect();
    if choices.is_empty() {
        choices.push(ChoiceEntry::new("Life"));
    }
    ChoiceOptions {
        choices,
        multiple: false,
        expanded: false,
    }
}

pub(super) fn build_post_bind_form(name: impl Into<String>, categories: &[String]) -> Form {
    Form::builder(name)
        .field_config(
            "body",
            FieldConfig::Hidden(HiddenOptions::default()),
            not_blank(),
        )
        .field("embed_url", vec![])
        .field_config("image", FieldConfig::File(image_file_options()), vec![])
        .field_config(
            "category",
            FieldConfig::Choice(category_choice_options(categories)),
            vec![],
        )
        .build()
}

fn build_composer_form(
    csrf: &HmacCsrfTokenManager,
    categories: &[String],
    form_name: &str,
    action: &str,
    post: Option<&Post>,
    submit_label: &str,
    cancel_href: &str,
) -> Result<String, HttpError> {
    let mut form = Form::builder(form_name)
        .action(action)
        .field_config(
            "body",
            FieldConfig::Hidden(HiddenOptions::default()),
            not_blank(),
        )
        .field("embed_url", vec![])
        .field_config("image", FieldConfig::File(image_file_options()), vec![])
        .field_config(
            "category",
            FieldConfig::Choice(category_choice_options(categories)),
            vec![],
        )
        .build();
    if let Some(post) = post {
        let _ = form.set("embed_url", post.embed_url.clone().unwrap_or_default());
        let _ = form.set("category", post.category.clone());
    } else if let Some(first) = categories.first() {
        let _ = form.set("category", first.clone());
    }
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = form_csrf(&form).map_err(|err| HttpError::failed(err.to_string()))?;
    let image_field = form
        .fields()
        .iter()
        .find(|field| field.name() == "image")
        .ok_or_else(|| HttpError::failed("missing image field"))?;
    let category_field = form
        .fields()
        .iter()
        .find(|field| field.name() == "category")
        .ok_or_else(|| HttpError::failed("missing category field"))?;
    let image_widget =
        form_widget(image_field).map_err(|err| HttpError::failed(err.to_string()))?;
    let category_widget =
        form_widget(category_field).map_err(|err| HttpError::failed(err.to_string()))?;
    let emojis = emoji_picker();
    let embed_value = post.and_then(|p| p.embed_url.as_deref()).unwrap_or("");
    let existing_preview =
        post.and_then(|p| p.image_data.as_deref())
            .map_or(String::new(), |data| {
                format!(
                    r#"<img class="img-fluid rounded border" alt="current media" src="{src}" />"#,
                    src = escape_attr(data)
                )
            });
    let initial_attr = post.map_or(String::new(), |p| {
        format!(
            r#" data-initial-html="{html}""#,
            html = escape_attr(&p.body)
        )
    });
    let cancel = if post.is_some() {
        format!(
            r#"<a href="{href}" class="btn btn-outline-secondary">Cancel</a>"#,
            href = escape_attr(cancel_href)
        )
    } else {
        r#"<button type="button" id="composer-cancel" class="btn btn-outline-secondary">Cancel</button>"#
            .to_owned()
    };
    Ok(format!(
        r#"<form name="{form_name}" method="POST" action="{action}" class="composer-form" enctype="multipart/form-data">
{csrf_field}
<input type="hidden" id="body" name="body" value="" />

<div class="mb-3 composer-emoji-wrap">
  <div id="composer-quill"{initial_attr}></div>
  <div class="d-flex justify-content-end mt-1">
    <span id="composer-count" class="text-secondary small">0/2000</span>
  </div>
  {emojis}
</div>

<div class="mb-3">
  <label class="form-label" for="image">Media (image) - optional</label>
  <div class="composer-file">{image_widget}</div>
  <div id="media-preview" class="mt-2">{existing_preview}</div>
  <label class="form-label mt-3" for="embed_url">…or paste a link instead</label>
  <input type="url" class="form-control" id="embed_url" name="embed_url" value="{embed_value}" placeholder="https://… (.mp4, YouTube, .jpg)" />
</div>

<div class="mb-3">
  <label class="form-label" for="category">Category</label>
  <div class="composer-choice">{category_widget}</div>
</div>

<div class="d-flex gap-2 justify-content-end">
  {cancel}
  <button type="submit" class="btn btn-primary px-4">{submit_label}</button>
</div>
</form>"#,
        embed_value = escape_html(embed_value),
        submit_label = escape_html(submit_label),
    ))
}

pub(super) fn build_admin_post_actions(
    csrf: &HmacCsrfTokenManager,
    routes: &RouteCollection,
    post_id: u64,
) -> Result<String, HttpError> {
    let id = post_id.to_string();
    let delete_action = path(routes, "admin_post_delete", &[("id", id.as_str())])?;
    let edit_href = path(routes, "admin_post_edit_get", &[("id", id.as_str())])?;
    let mut form = Form::builder(format!("delete-{post_id}"))
        .action(delete_action.clone())
        .build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    let csrf_token = extract_csrf_value(&csrf_field);
    Ok(format!(
        r#"<a class="btn btn-sm btn-outline-primary" href="{edit}">Edit</a>
<button type="button" class="btn btn-sm btn-outline-danger" data-clitorine-delete data-delete-action="{delete}" data-delete-token="{token}">Delete</button>"#,
        edit = escape_attr(&edit_href),
        delete = escape_attr(&delete_action),
        token = escape_attr(&csrf_token),
    ))
}

fn extract_csrf_value(csrf_hidden: &str) -> String {
    let key = "value=\"";
    let Some(start) = csrf_hidden.find(key) else {
        return String::new();
    };
    csrf_hidden[start + key.len()..]
        .split('"')
        .next()
        .unwrap_or("")
        .to_owned()
}

pub(super) fn build_comment_form(
    csrf: &HmacCsrfTokenManager,
    routes: &RouteCollection,
    post_id: u64,
) -> Result<String, HttpError> {
    let name = format!("comment-{post_id}");
    let id = post_id.to_string();
    let action = path(routes, "comment_create", &[("id", id.as_str())])?;
    let mut form = Form::builder(name)
        .action(action.clone())
        .field_config(
            "body",
            FieldConfig::Textarea(TextareaOptions {
                placeholder: Some("Write a comment…".into()),
                rows: Some(2),
            }),
            not_blank(),
        )
        .build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = form_csrf(&form).map_err(|err| HttpError::failed(err.to_string()))?;
    let body_field = form
        .fields()
        .iter()
        .find(|field| field.name() == "body")
        .ok_or_else(|| HttpError::failed("missing body field"))?;
    let body_widget = form_widget(body_field).map_err(|err| HttpError::failed(err.to_string()))?;
    Ok(format!(
        r#"<form method="POST" action="{action}" class="mt-2" data-clitorine-ajax="comment">
{csrf_field}
<label class="form-label" for="body">Add a comment (needs approval)</label>
<div class="composer-textarea">{body_widget}</div>
<button type="submit" class="btn btn-sm btn-outline-primary mt-2">Submit comment</button>
</form>"#,
        action = escape_attr(&action),
    ))
}

pub(super) fn build_like_form(
    csrf: &HmacCsrfTokenManager,
    routes: &RouteCollection,
    post_id: u64,
) -> Result<String, HttpError> {
    let name = format!("like-{post_id}");
    let id = post_id.to_string();
    let action = path(routes, "post_like", &[("id", id.as_str())])?;
    let mut form = Form::builder(name).action(action.clone()).build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    Ok(format!(
        r#"<form method="POST" action="{action}" class="like-form" data-clitorine-ajax="like">{csrf_field}<button type="submit" class="btn btn-sm btn-outline-secondary">Like</button></form>"#,
        action = escape_attr(&action),
    ))
}

pub(super) fn build_admin_action_form(
    csrf: &HmacCsrfTokenManager,
    form_name: &str,
    action: &str,
    label: &str,
    class: &str,
) -> Result<String, HttpError> {
    let mut form = Form::builder(form_name).action(action).build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    Ok(format!(
        r#"<form method="POST" action="{action}" class="d-inline">{csrf_field}<button type="submit" class="{class}">{label}</button></form>"#,
        action = escape_attr(action),
        label = escape_html(label),
        class = escape_attr(class),
    ))
}

pub(super) fn build_login_form(
    csrf: &HmacCsrfTokenManager,
    routes: &RouteCollection,
) -> Result<String, HttpError> {
    let action = path(routes, "admin_login", &[])?;
    let mut form = Form::builder("admin-login")
        .action(action.clone())
        .field("token", not_blank())
        .build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    Ok(format!(
        r#"<form method="POST" action="{action}">
{csrf_field}
<label class="form-label" for="token">Admin token</label>
<input class="form-control" type="password" id="token" name="token" required autocomplete="current-password" />
<button type="submit" class="btn btn-primary mt-3">Open queue</button>
</form>"#,
        action = escape_attr(&action),
    ))
}

pub(super) fn build_logout_form(
    csrf: &HmacCsrfTokenManager,
    routes: &RouteCollection,
) -> Result<String, HttpError> {
    let action = path(routes, "admin_logout", &[])?;
    let mut form = Form::builder("admin-logout").action(action.clone()).build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    Ok(format!(
        r#"<form method="POST" action="{action}" class="d-inline">{csrf_field}<button type="submit" class="btn btn-outline-secondary btn-sm">Log out</button></form>"#,
        action = escape_attr(&action),
    ))
}

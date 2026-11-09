use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serenade_form::{Form, FormStatus, UploadedFile};
use serenade_http::{HttpError, Request, Response};

use crate::embed::is_allowed_embed;
use crate::feed::feed_with_flash;
use crate::forms::{build_edit_form, build_login_form, build_post_bind_form};
use crate::html::{admin_login_page, edit_post_page, html_response, redirect};
use crate::sanitize::{plain_len, sanitize_post_html};
use crate::search::{index_delete, index_upsert};
use crate::state::{AppState, MAX_BODY_CHARS, MAX_IMAGE_BYTES, MAX_IMAGE_DATA, is_admin, ui_for};
use crate::store::NewPost;

fn sanitize_image_data(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.len() > MAX_IMAGE_DATA {
        return None;
    }
    if !(raw.starts_with("data:image/png;base64,")
        || raw.starts_with("data:image/jpeg;base64,")
        || raw.starts_with("data:image/jpg;base64,")
        || raw.starts_with("data:image/webp;base64,")
        || raw.starts_with("data:image/gif;base64,"))
    {
        return None;
    }
    if !raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | ';' | ',' | '+' | '=' | '_'))
    {
        return None;
    }
    Some(raw.to_owned())
}

fn uploaded_image_to_data_url(file: &UploadedFile) -> Result<String, &'static str> {
    let bytes = file
        .as_bytes()
        .ok_or("Image upload rejected (type or size).")?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err("Image upload rejected (type or size).");
    }
    let mime = file.content_type().unwrap_or("");
    let mime = match mime {
        "image/png" | "image/jpeg" | "image/jpg" | "image/webp" | "image/gif" => mime,
        _ => return Err("Image upload rejected (type or size)."),
    };
    let encoded = BASE64.encode(bytes);
    let data = format!("data:{mime};base64,{encoded}");
    sanitize_image_data(&data).ok_or("Image upload rejected (type or size).")
}

fn parse_new_post(
    state: &AppState,
    form: &Form,
    existing_image: Option<&str>,
) -> Result<NewPost, &'static str> {
    let raw_body = form.get("body").unwrap_or("").trim();
    let body = sanitize_post_html(raw_body);
    if plain_len(&body) == 0 || plain_len(&body) > MAX_BODY_CHARS {
        return Err("Post body is required (max 2000 characters).");
    }
    let embed_raw = form.get("embed_url").unwrap_or("").trim();
    let embed_url = if embed_raw.is_empty() {
        None
    } else if is_allowed_embed(embed_raw) {
        Some(embed_raw.to_owned())
    } else {
        return Err("Media URL must be YouTube, SoundCloud, image, or .mp4.");
    };
    let image_data = if let Some(file) = form.file("image") {
        Some(uploaded_image_to_data_url(file)?)
    } else if let Some(existing) = existing_image {
        sanitize_image_data(existing)
    } else {
        None
    };
    let category = form.get("category").unwrap_or("").trim();
    let category = if state.store.has_category(category) {
        category.to_owned()
    } else {
        state
            .store
            .categories()
            .into_iter()
            .next()
            .unwrap_or_else(|| "Life".to_owned())
    };
    Ok(NewPost {
        body,
        embed_url,
        image_data,
        category,
    })
}

pub(super) fn handle_post_create(
    state: &AppState,
    request: &Request,
) -> Result<Response, HttpError> {
    let categories = state.store.categories();
    let mut form = build_post_bind_form("post", &categories);
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => match parse_new_post(state, &form, None) {
            Ok(new) => {
                let post = state.store.add_post(new);
                index_upsert(state, &post);
                Ok(redirect("/"))
            }
            Err(msg) => feed_with_flash(state, request, Some(msg), true, true),
        },
        Ok(FormStatus::Bound) => {
            feed_with_flash(state, request, Some("Post body is required."), true, true)
        }
        Ok(FormStatus::NotSubmitted) => Ok(redirect("/")),
        Err(_) => feed_with_flash(
            state,
            request,
            Some("Invalid form or CSRF token. Reload and try again."),
            true,
            true,
        ),
    }
}

fn route_post_id(request: &Request) -> Result<u64, HttpError> {
    request
        .attributes()
        .get::<String>("id")
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| HttpError::bad_request("bad post id"))
}

pub(super) fn handle_post_edit_get(
    state: &AppState,
    request: &Request,
) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf, state.matcher.collection())?;
        return Ok(html_response(
            401,
            admin_login_page(
                &ui_for(state, request),
                state.matcher.collection(),
                &login,
                Some("Sign in first."),
            ),
        ));
    }
    let id = route_post_id(request)?;
    let Some(post) = state.store.get_post(id) else {
        return Err(HttpError::not_found("post missing"));
    };
    let categories = state.store.categories();
    let form = build_edit_form(&state.csrf, state.matcher.collection(), &categories, &post)?;
    Ok(html_response(
        200,
        edit_post_page(
            &ui_for(state, request),
            state.matcher.collection(),
            id,
            &form,
            None,
        ),
    ))
}

pub(super) fn handle_post_edit_post(
    state: &AppState,
    request: &Request,
) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf, state.matcher.collection())?;
        return Ok(html_response(
            401,
            admin_login_page(
                &ui_for(state, request),
                state.matcher.collection(),
                &login,
                Some("Sign in first."),
            ),
        ));
    }
    let id = route_post_id(request)?;
    let Some(post) = state.store.get_post(id) else {
        return Err(HttpError::not_found("post missing"));
    };
    let form_name = format!("edit-{id}");
    let categories = state.store.categories();
    let mut form = build_post_bind_form(form_name, &categories);
    let existing_image = post.image_data.clone();
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => {
            match parse_new_post(state, &form, existing_image.as_deref()) {
                Ok(new) => {
                    let _ = state.store.update_post(id, &new);
                    if let Some(post) = state.store.get_post(id) {
                        index_upsert(state, &post);
                    }
                    Ok(redirect(&format!("/#post-{id}")))
                }
                Err(msg) => {
                    let categories = state.store.categories();
                    let form_html = build_edit_form(
                        &state.csrf,
                        state.matcher.collection(),
                        &categories,
                        &post,
                    )?;
                    Ok(html_response(
                        400,
                        edit_post_page(
                            &ui_for(state, request),
                            state.matcher.collection(),
                            id,
                            &form_html,
                            Some(msg),
                        ),
                    ))
                }
            }
        }
        _ => Ok(redirect(&format!("/admin/posts/{id}/edit"))),
    }
}

pub(super) fn handle_post_delete(
    state: &AppState,
    request: &Request,
) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf, state.matcher.collection())?;
        return Ok(html_response(
            401,
            admin_login_page(
                &ui_for(state, request),
                state.matcher.collection(),
                &login,
                Some("Sign in first."),
            ),
        ));
    }
    let id = route_post_id(request)?;
    let mut form = Form::builder(format!("delete-{id}")).build();
    if form.handle_request(request, &state.csrf).is_err() {
        return Ok(redirect("/"));
    }
    let _ = state.store.delete_post(id);
    index_delete(state, id);
    Ok(redirect("/"))
}

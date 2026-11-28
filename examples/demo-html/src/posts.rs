use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serenade_form::{Form, FormStatus, UploadedFile};
use serenade_http::{HttpError, Request, Response};
use serenade_view::path;

use crate::embed::is_allowed_embed;
use crate::feed::feed_with_flash;
use crate::forms::{build_edit_form, build_login_form, build_post_bind_form};
use crate::html::{admin_login_page, edit_post_page, html_response, redirect};
use crate::preview;
use crate::sanitize::{plain_len, sanitize_post_html};
use crate::search::{index_delete, index_upsert};
use crate::state::{AppState, MAX_BODY_CHARS, MAX_IMAGE_BYTES, MAX_IMAGE_DATA, is_admin, ui_for};
use crate::store::NewPost;
use std::hash::{Hash, Hasher};

pub fn handle_post_image(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let id = route_post_id(request)?;
    let data = state
        .store
        .image_data(id)
        .map_err(|err| {
            tracing::error!(%err, "read post image");
            HttpError::failed("Could not load image.")
        })?
        .ok_or_else(|| HttpError::not_found("image missing"))?;
    let (mime, bytes) = decode_image(&data)?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hash);
    mime.hash(&mut hash);
    let etag = format!("\"{:x}\"", hash.finish());
    let response = if request
        .headers()
        .get("if-none-match")
        .is_some_and(|value| value == etag)
    {
        Response::new(304)
    } else {
        Response::new(200)
            .with_header("content-type", mime)
            .with_body(bytes)
    };
    Ok(response
        .with_header("etag", etag)
        .with_header("cache-control", "no-cache")
        .with_header("x-content-type-options", "nosniff"))
}

fn decode_image(data: &str) -> Result<(&str, Vec<u8>), HttpError> {
    let fail = || {
        tracing::error!("Invalid stored MyFeed image");
        HttpError::failed("Invalid stored image.")
    };
    if data.len() > MAX_IMAGE_DATA {
        return Err(fail());
    }
    let (prefix, encoded) = data.split_once(";base64,").ok_or_else(fail)?;
    let mime = prefix.strip_prefix("data:").ok_or_else(fail)?;
    if !matches!(
        mime,
        "image/png" | "image/jpeg" | "image/jpg" | "image/webp" | "image/gif"
    ) {
        return Err(fail());
    }
    let bytes = BASE64.decode(encoded).map_err(|_| fail())?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(fail());
    }
    Ok((mime, bytes))
}

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
    let embed_raw = form.get("embed_url").unwrap_or("").trim();
    let embed_url = if embed_raw.is_empty() {
        None
    } else if preview::web_url(embed_raw).is_some() {
        Some(embed_raw.to_owned())
    } else {
        return Err("Link must be a valid HTTP or HTTPS URL.");
    };
    if (plain_len(&body) == 0 && embed_url.is_none()) || plain_len(&body) > MAX_BODY_CHARS {
        return Err("Post text or a link is required (max 2000 characters).");
    }
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

async fn cache_article_preview(state: &AppState, new: &NewPost) -> Result<(), HttpError> {
    let Some(url) = new
        .embed_url
        .as_deref()
        .filter(|url| !is_allowed_embed(url))
    else {
        return Ok(());
    };
    let database_error = |err: rusqlite::Error| {
        tracing::error!(%err, "MyFeed article preview database failure");
        HttpError::failed("Could not save article preview.")
    };
    if state
        .store
        .link_preview(url)
        .map_err(database_error)?
        .is_some()
    {
        return Ok(());
    }
    match preview::fetch(url).await {
        Ok(preview) => state
            .store
            .save_link_preview(url, &preview)
            .map_err(database_error)?,
        Err(err) => tracing::warn!(%err, "Article preview unavailable; retaining clickable link"),
    }
    Ok(())
}

pub async fn handle_post_create(
    state: &AppState,
    request: &Request,
) -> Result<Response, HttpError> {
    let categories = state.store.categories();
    let mut form = build_post_bind_form("post", &categories);
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => match parse_new_post(state, &form, None) {
            Ok(new) => {
                cache_article_preview(state, &new).await?;
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

pub fn route_post_id(request: &Request) -> Result<u64, HttpError> {
    request
        .attributes()
        .get::<String>("id")
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|id| *id > 0 && i64::try_from(*id).is_ok())
        .ok_or_else(|| HttpError::bad_request("bad post id"))
}

pub fn handle_post_edit_get(state: &AppState, request: &Request) -> Result<Response, HttpError> {
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

pub async fn handle_post_edit_post(
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
                    cache_article_preview(state, &new).await?;
                    let _ = state.store.update_post(id, &new);
                    if let Some(post) = state.store.get_post(id) {
                        index_upsert(state, &post);
                    }
                    Ok(redirect(&format!("/posts/{id}")))
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
        _ => Ok(redirect(&path(
            state.matcher.collection(),
            "admin_post_edit_get",
            &[("id", &id.to_string())],
        )?)),
    }
}

pub fn handle_post_delete(state: &AppState, request: &Request) -> Result<Response, HttpError> {
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

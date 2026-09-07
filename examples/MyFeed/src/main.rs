//! `MyFeed`: open public feed demo (Serenade Form + CSRF, Actix listen).
//!
//! Bootstrap + Quill for UI; Clitorine (`assets/clitorine.js`) wires composer helpers.
//! Run: `cargo run -p my_feed`

mod embed;
mod emoji;
mod html;
mod sanitize;
mod store;

use std::fmt::Write as _;
use std::sync::Arc;

use serenade_form::{Form, FormStatus, escape_attr, escape_html};
use serenade_http::{
    AsyncHttpKernel, HttpError, Method, ROUTE_ATTRIBUTE, Request, Response, Route, RouteCollection,
    UrlMatcher,
};
use serenade_security::HmacCsrfTokenManager;
use serenade_validator::NotBlank;

use crate::embed::is_allowed_embed;
use crate::html::{
    FeedView, admin_login_page, admin_page_with_logout, asset_response, edit_post_page,
    emoji_picker, feed_page, html_response, redirect, redirect_with_cookie,
};
use crate::sanitize::{plain_len, sanitize_post_html};
use crate::store::{FeedStore, NewPost, Post};

const DEFAULT_BIND: &str = "127.0.0.1:8090";
const DEFAULT_CSRF: &str = "myfeed-dev-csrf-secret-change-me!!";
const DEFAULT_ADMIN: &str = "myfeed-dev-admin";
const ADMIN_COOKIE: &str = "myfeed_admin";
const MAX_IMAGE_DATA: usize = 280_000;
const MAX_BODY_CHARS: usize = 2000;

struct AppState {
    store: FeedStore,
    csrf: HmacCsrfTokenManager,
    matcher: UrlMatcher,
    admin_token: String,
}

fn routes() -> Result<RouteCollection, HttpError> {
    let mut collection = RouteCollection::new();
    collection.add(Route::with_method("feed", "/", Method::Get))?;
    collection.add(Route::with_method("post_create", "/posts", Method::Post))?;
    collection.add(Route::with_method(
        "comment_create",
        "/posts/{id}/comments",
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "post_like",
        "/posts/{id}/like",
        Method::Post,
    ))?;
    collection.add(Route::with_method("admin", "/admin", Method::Get))?;
    collection.add(Route::with_method(
        "admin_login",
        "/admin/login",
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_logout",
        "/admin/logout",
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_approve",
        "/admin/comments/{id}/approve",
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_reject",
        "/admin/comments/{id}/reject",
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_category_add",
        "/admin/categories",
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_category_delete",
        "/admin/categories/delete",
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_post_edit_get",
        "/admin/posts/{id}/edit",
        Method::Get,
    ))?;
    collection.add(Route::with_method(
        "admin_post_edit_post",
        "/admin/posts/{id}/edit",
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_post_delete",
        "/admin/posts/{id}/delete",
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "asset_css",
        "/assets/myfeed.css",
        Method::Get,
    ))?;
    collection.add(Route::with_method(
        "asset_js",
        "/assets/clitorine.js",
        Method::Get,
    ))?;
    Ok(collection)
}

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

fn not_blank() -> Vec<Arc<dyn serenade_validator::Constraint>> {
    vec![Arc::new(NotBlank) as Arc<dyn serenade_validator::Constraint>]
}

fn build_post_form(
    csrf: &HmacCsrfTokenManager,
    categories: &[String],
) -> Result<String, HttpError> {
    build_composer_form(csrf, categories, "post", "/posts", None, "Post")
}

fn build_edit_form(
    csrf: &HmacCsrfTokenManager,
    categories: &[String],
    post: &Post,
) -> Result<String, HttpError> {
    let name = format!("edit-{}", post.id);
    let action = format!("/admin/posts/{}/edit", post.id);
    build_composer_form(csrf, categories, &name, &action, Some(post), "Save changes")
}

fn build_composer_form(
    csrf: &HmacCsrfTokenManager,
    categories: &[String],
    form_name: &str,
    action: &str,
    post: Option<&Post>,
    submit_label: &str,
) -> Result<String, HttpError> {
    let mut form = Form::builder(form_name)
        .action(action)
        .field("body", not_blank())
        .field("embed_url", vec![])
        .field("image_data", vec![])
        .field("category", vec![])
        .build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    let emojis = emoji_picker();
    let selected_category = post.map_or("", |p| p.category.as_str());
    let mut options = String::new();
    for (index, name) in categories.iter().enumerate() {
        let selected = if (!selected_category.is_empty() && name == selected_category)
            || (selected_category.is_empty() && index == 0)
        {
            " selected"
        } else {
            ""
        };
        let _ = write!(
            options,
            r#"<option value="{value}"{selected}>{label}</option>"#,
            value = escape_html(name),
            label = escape_html(name),
        );
    }
    if options.is_empty() {
        options.push_str(r#"<option value="Life">Life</option>"#);
    }
    let embed_value = post.and_then(|p| p.embed_url.as_deref()).unwrap_or("");
    let image_value = post.and_then(|p| p.image_data.as_deref()).unwrap_or("");
    let initial_attr = post.map_or(String::new(), |p| {
        format!(
            r#" data-initial-html="{html}""#,
            html = escape_attr(&p.body)
        )
    });
    let cancel = if post.is_some() {
        r#"<a href="/" class="btn btn-outline-secondary">Cancel</a>"#
    } else {
        r#"<button type="button" id="composer-cancel" class="btn btn-outline-secondary">Cancel</button>"#
    };
    Ok(format!(
        r#"<form name="{form_name}" method="POST" action="{action}" class="composer-form">
{csrf_field}
<input type="hidden" id="body" name="body" value="" />
<input type="hidden" id="image_data" name="image_data" value="{image_value}" />

<div class="mb-3 composer-emoji-wrap">
  <div id="composer-quill"{initial_attr}></div>
  <div class="d-flex justify-content-end mt-1">
    <span id="composer-count" class="text-secondary small">0/2000</span>
  </div>
  {emojis}
</div>

<div class="mb-3">
  <label class="form-label" for="media_upload">Media (image) - optional</label>
  <input type="file" id="media_upload" class="form-control" accept="image/*" />
  <div id="media-preview" class="mt-2"></div>
  <label class="form-label mt-3" for="embed_url">…or paste a link instead</label>
  <input type="url" class="form-control" id="embed_url" name="embed_url" value="{embed_value}" placeholder="https://… (.mp4, YouTube, .jpg)" />
</div>

<div class="mb-3">
  <label class="form-label" for="category">Category</label>
  <select class="form-select" id="category" name="category">
    {options}
  </select>
</div>

<div class="d-flex gap-2 justify-content-end">
  {cancel}
  <button type="submit" class="btn btn-primary px-4">{submit_label}</button>
</div>
</form>"#,
        image_value = escape_html(image_value),
        embed_value = escape_html(embed_value),
        submit_label = escape_html(submit_label),
    ))
}

fn build_admin_post_actions(
    csrf: &HmacCsrfTokenManager,
    post_id: u64,
) -> Result<String, HttpError> {
    let mut form = Form::builder(format!("delete-{post_id}"))
        .action(format!("/admin/posts/{post_id}/delete"))
        .build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    let csrf_token = extract_csrf_value(&csrf_field);
    Ok(format!(
        r#"<a class="btn btn-sm btn-outline-primary" href="/admin/posts/{post_id}/edit">Edit</a>
<button type="button" class="btn btn-sm btn-outline-danger" data-clitorine-delete data-delete-action="/admin/posts/{post_id}/delete" data-delete-token="{token}">Delete</button>"#,
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

fn build_comment_form(csrf: &HmacCsrfTokenManager, post_id: u64) -> Result<String, HttpError> {
    let name = format!("comment-{post_id}");
    let mut form = Form::builder(name)
        .action(format!("/posts/{post_id}/comments"))
        .field("body", not_blank())
        .build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    Ok(format!(
        r#"<form method="POST" action="/posts/{post_id}/comments" class="mt-2" data-clitorine-ajax="comment">
{csrf_field}
<label class="form-label" for="cbody-{post_id}">Add a comment (needs approval)</label>
<textarea class="form-control" id="cbody-{post_id}" name="body" maxlength="1000" rows="2" required></textarea>
<button type="submit" class="btn btn-sm btn-outline-primary mt-2">Submit comment</button>
</form>"#
    ))
}

fn build_like_form(csrf: &HmacCsrfTokenManager, post_id: u64) -> Result<String, HttpError> {
    let name = format!("like-{post_id}");
    let mut form = Form::builder(name)
        .action(format!("/posts/{post_id}/like"))
        .build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    Ok(format!(
        r#"<form method="POST" action="/posts/{post_id}/like" class="like-form" data-clitorine-ajax="like">{csrf_field}<button type="submit" class="btn btn-sm btn-outline-secondary">Like</button></form>"#
    ))
}

fn build_admin_action_form(
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
        r#"<form method="POST" action="{action}" class="d-inline">{csrf_field}<button type="submit" class="{class}">{label}</button></form>"#
    ))
}

fn build_login_form(csrf: &HmacCsrfTokenManager) -> Result<String, HttpError> {
    let mut form = Form::builder("admin-login")
        .action("/admin/login")
        .field("token", not_blank())
        .build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    Ok(format!(
        r#"<form method="POST" action="/admin/login">
{csrf_field}
<label class="form-label" for="token">Admin token</label>
<input class="form-control" type="password" id="token" name="token" required autocomplete="current-password" />
<button type="submit" class="btn btn-primary mt-3">Open queue</button>
</form>"#
    ))
}

fn build_logout_form(csrf: &HmacCsrfTokenManager) -> Result<String, HttpError> {
    let mut form = Form::builder("admin-logout")
        .action("/admin/logout")
        .build();
    form.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let full = form
        .render()
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let csrf_field = extract_csrf_hidden(full.as_html());
    Ok(format!(
        r#"<form method="POST" action="/admin/logout" class="d-inline">{csrf_field}<button type="submit" class="btn btn-outline-secondary btn-sm">Log out</button></form>"#
    ))
}

fn build_categories_admin(
    csrf: &HmacCsrfTokenManager,
    categories: &[String],
) -> Result<String, HttpError> {
    let mut add = Form::builder("category-add")
        .action("/admin/categories")
        .field("name", not_blank())
        .build();
    add.prepare_csrf(csrf)
        .map_err(|err| HttpError::failed(err.to_string()))?;
    let add_csrf = extract_csrf_hidden(
        add.render()
            .map_err(|err| HttpError::failed(err.to_string()))?
            .as_html(),
    );

    let mut list = String::new();
    for name in categories {
        let mut del = Form::builder("category-delete")
            .action("/admin/categories/delete")
            .field("name", not_blank())
            .build();
        del.prepare_csrf(csrf)
            .map_err(|err| HttpError::failed(err.to_string()))?;
        let del_csrf = extract_csrf_hidden(
            del.render()
                .map_err(|err| HttpError::failed(err.to_string()))?
                .as_html(),
        );
        let _ = write!(
            list,
            r#"<span class="category-chip">{label}
<form method="POST" action="/admin/categories/delete" class="d-inline">
{del_csrf}
<input type="hidden" name="name" value="{value}" />
<button type="submit" class="btn btn-sm btn-link text-danger p-0">Remove</button>
</form>
</span>"#,
            label = escape_html(name),
            value = escape_html(name),
        );
    }

    Ok(format!(
        r#"<div class="card post-card"><div class="card-body">
<p class="text-secondary small">Natural labels for the composer. Visitors pick one when posting.</p>
<div class="mb-3">{list}</div>
<form method="POST" action="/admin/categories" class="row g-2 align-items-end">
{add_csrf}
<div class="col">
  <label class="form-label" for="cat-name">Add category</label>
  <input class="form-control" id="cat-name" name="name" maxlength="40" required />
</div>
<div class="col-auto">
  <button type="submit" class="btn btn-primary">Add</button>
</div>
</form>
</div></div>"#
    ))
}

fn cookie_value(request: &Request, name: &str) -> Option<String> {
    let header = request.headers().get("cookie")?;
    for part in header.split(';') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            if key.trim() == name {
                return Some(value.trim().to_owned());
            }
        }
    }
    None
}

fn is_admin(request: &Request, expected: &str) -> bool {
    if request.headers().get("Authorization").is_some_and(|value| {
        let key = value.strip_prefix("Bearer ").unwrap_or(value);
        key == expected
    }) {
        return true;
    }
    cookie_value(request, ADMIN_COOKIE).is_some_and(|value| value == expected)
}

fn feed_with_flash(
    state: &AppState,
    request: &Request,
    flash: Option<&str>,
    is_err: bool,
    composer_open: bool,
) -> Result<Response, HttpError> {
    let categories = state.store.categories();
    let post_form = build_post_form(&state.csrf, &categories)?;
    let mut comment_forms = Vec::new();
    let mut like_forms = Vec::new();
    let mut admin_forms = Vec::new();
    let admin = is_admin(request, &state.admin_token);
    for post in state.store.posts() {
        comment_forms.push((post.id, build_comment_form(&state.csrf, post.id)?));
        like_forms.push((post.id, build_like_form(&state.csrf, post.id)?));
        if admin {
            admin_forms.push((post.id, build_admin_post_actions(&state.csrf, post.id)?));
        }
    }
    Ok(html_response(
        if is_err { 400 } else { 200 },
        feed_page(&FeedView {
            store: &state.store,
            post_form_html: &post_form,
            comment_forms: &comment_forms,
            like_forms: &like_forms,
            admin_forms: &admin_forms,
            flash,
            flash_err: is_err,
            composer_open,
        }),
    ))
}

fn handle(state: &AppState, request: &mut Request) -> Result<Response, HttpError> {
    state.matcher.apply(request)?;
    let route = request
        .attributes()
        .get::<String>(ROUTE_ATTRIBUTE)
        .map_or("", String::as_str);

    match route {
        "feed" => feed_with_flash(state, request, None, false, false),
        "post_create" => handle_post_create(state, request),
        "comment_create" => handle_comment_create(state, request),
        "post_like" => handle_like(state, request),
        "admin" => handle_admin_get(state, request),
        "admin_login" => handle_admin_login(state, request),
        "admin_logout" => Ok(handle_admin_logout(state, request)),
        "admin_approve" => handle_admin_moderation(state, request, true),
        "admin_reject" => handle_admin_moderation(state, request, false),
        "admin_category_add" => handle_category_add(state, request),
        "admin_category_delete" => handle_category_delete(state, request),
        "admin_post_edit_get" => handle_post_edit_get(state, request),
        "admin_post_edit_post" => handle_post_edit_post(state, request),
        "admin_post_delete" => handle_post_delete(state, request),
        "asset_css" => Ok(asset_response(
            "text/css; charset=utf-8",
            include_bytes!("../assets/myfeed.css"),
        )),
        "asset_js" => Ok(asset_response(
            "text/javascript; charset=utf-8",
            include_bytes!("../assets/clitorine.js"),
        )),
        _ => Err(HttpError::not_found("no handler")),
    }
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

fn parse_new_post(state: &AppState, form: &Form) -> Result<NewPost, &'static str> {
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
    let image_raw = form.get("image_data").unwrap_or("");
    let image_data = if image_raw.trim().is_empty() {
        None
    } else if let Some(data) = sanitize_image_data(image_raw) {
        Some(data)
    } else {
        return Err("Image upload rejected (type or size).");
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

fn handle_post_create(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let mut form = Form::builder("post")
        .field("body", not_blank())
        .field("embed_url", vec![])
        .field("image_data", vec![])
        .field("category", vec![])
        .build();
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => match parse_new_post(state, &form) {
            Ok(new) => {
                state.store.add_post(new);
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

fn wants_ajax(request: &Request) -> bool {
    request
        .headers()
        .get("x-myfeed-ajax")
        .is_some_and(|value| value == "1")
}

fn ajax_text(body: impl Into<String>) -> Response {
    Response::new(200)
        .with_header("content-type", "text/plain; charset=utf-8")
        .with_header("cache-control", "no-store")
        .with_body(body.into().into_bytes())
}

fn handle_comment_create(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let post_id = request
        .attributes()
        .get::<String>("id")
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| HttpError::bad_request("bad post id"))?;
    let form_name = format!("comment-{post_id}");
    let mut form = Form::builder(form_name).field("body", not_blank()).build();
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => {
            let body = form.get("body").unwrap_or("").trim().to_owned();
            if state.store.add_comment(post_id, body).is_none() {
                return Err(HttpError::not_found("post missing"));
            }
            if wants_ajax(request) {
                return Ok(ajax_text("Comment sent. It will appear after approval."));
            }
            feed_with_flash(
                state,
                request,
                Some("Comment sent. It will appear after approval."),
                false,
                false,
            )
        }
        _ if wants_ajax(request) => Ok(ajax_text("Could not send comment.")),
        _ => Ok(redirect("/")),
    }
}

fn handle_like(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let post_id = request
        .attributes()
        .get::<String>("id")
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| HttpError::bad_request("bad post id"))?;
    let form_name = format!("like-{post_id}");
    let mut form = Form::builder(form_name).build();
    if form.handle_request(request, &state.csrf).is_err() {
        if wants_ajax(request) {
            return Ok(Response::new(400)
                .with_header("content-type", "text/plain; charset=utf-8")
                .with_body(b"bad csrf".to_vec()));
        }
        return Ok(redirect("/"));
    }
    let likes = state.store.like_post(post_id).unwrap_or(0);
    if wants_ajax(request) {
        return Ok(ajax_text(likes.to_string()));
    }
    Ok(redirect(&format!("/#post-{post_id}")))
}

fn handle_admin_get(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf)?;
        return Ok(html_response(200, admin_login_page(&login, None)));
    }
    render_admin(state, None)
}

fn handle_admin_login(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let mut form = Form::builder("admin-login")
        .field("token", not_blank())
        .build();
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => {
            let token = form.get("token").unwrap_or("").trim();
            if token != state.admin_token {
                let login = build_login_form(&state.csrf)?;
                return Ok(html_response(
                    401,
                    admin_login_page(&login, Some("Invalid admin token.")),
                ));
            }
            let cookie =
                format!("{ADMIN_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400");
            Ok(redirect_with_cookie("/admin", &cookie))
        }
        _ => {
            let login = build_login_form(&state.csrf)?;
            Ok(html_response(
                400,
                admin_login_page(&login, Some("Login failed. Try again.")),
            ))
        }
    }
}

fn handle_admin_logout(state: &AppState, request: &Request) -> Response {
    let mut form = Form::builder("admin-logout").build();
    let _ = form.handle_request(request, &state.csrf);
    let cookie = format!("{ADMIN_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    redirect_with_cookie("/", &cookie)
}

fn render_admin(state: &AppState, notice: Option<&str>) -> Result<Response, HttpError> {
    let pending = state.store.pending_comments();
    let mut forms = Vec::new();
    for comment in &pending {
        let approve = build_admin_action_form(
            &state.csrf,
            &format!("approve-{}", comment.id),
            &format!("/admin/comments/{}/approve", comment.id),
            "Approve",
            "btn btn-success btn-sm",
        )?;
        let reject = build_admin_action_form(
            &state.csrf,
            &format!("reject-{}", comment.id),
            &format!("/admin/comments/{}/reject", comment.id),
            "Reject",
            "btn btn-outline-danger btn-sm",
        )?;
        forms.push((comment.id, approve, reject));
    }
    let logout = build_logout_form(&state.csrf)?;
    let categories = state.store.categories();
    let categories_html = build_categories_admin(&state.csrf, &categories)?;
    Ok(html_response(
        200,
        admin_page_with_logout(&pending, &forms, &logout, &categories_html, notice),
    ))
}

fn handle_admin_moderation(
    state: &AppState,
    request: &Request,
    approve: bool,
) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf)?;
        return Ok(html_response(
            401,
            admin_login_page(&login, Some("Sign in first.")),
        ));
    }
    let id = request
        .attributes()
        .get::<String>("id")
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| HttpError::bad_request("bad comment id"))?;
    let form_name = if approve {
        format!("approve-{id}")
    } else {
        format!("reject-{id}")
    };
    let mut form = Form::builder(form_name).build();
    if form.handle_request(request, &state.csrf).is_err() {
        return Ok(redirect("/admin"));
    }
    if approve {
        let _ = state.store.approve_comment(id);
    } else {
        let _ = state.store.remove_comment(id);
    }
    Ok(redirect("/admin"))
}

fn handle_category_add(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf)?;
        return Ok(html_response(
            401,
            admin_login_page(&login, Some("Sign in first.")),
        ));
    }
    let mut form = Form::builder("category-add")
        .field("name", not_blank())
        .build();
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => {
            let name = form.get("name").unwrap_or("").trim();
            match state.store.add_category(name) {
                Ok(()) => Ok(redirect("/admin")),
                Err(msg) => render_admin(state, Some(msg)),
            }
        }
        _ => Ok(redirect("/admin")),
    }
}

fn handle_category_delete(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf)?;
        return Ok(html_response(
            401,
            admin_login_page(&login, Some("Sign in first.")),
        ));
    }
    let mut form = Form::builder("category-delete")
        .field("name", not_blank())
        .build();
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => {
            let name = form.get("name").unwrap_or("").trim();
            let _ = state.store.remove_category(name);
            Ok(redirect("/admin"))
        }
        _ => Ok(redirect("/admin")),
    }
}

fn route_post_id(request: &Request) -> Result<u64, HttpError> {
    request
        .attributes()
        .get::<String>("id")
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| HttpError::bad_request("bad post id"))
}

fn handle_post_edit_get(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf)?;
        return Ok(html_response(
            401,
            admin_login_page(&login, Some("Sign in first.")),
        ));
    }
    let id = route_post_id(request)?;
    let Some(post) = state.store.get_post(id) else {
        return Err(HttpError::not_found("post missing"));
    };
    let categories = state.store.categories();
    let form = build_edit_form(&state.csrf, &categories, &post)?;
    Ok(html_response(200, edit_post_page(id, &form, None)))
}

fn handle_post_edit_post(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf)?;
        return Ok(html_response(
            401,
            admin_login_page(&login, Some("Sign in first.")),
        ));
    }
    let id = route_post_id(request)?;
    let Some(post) = state.store.get_post(id) else {
        return Err(HttpError::not_found("post missing"));
    };
    let form_name = format!("edit-{id}");
    let mut form = Form::builder(form_name)
        .field("body", not_blank())
        .field("embed_url", vec![])
        .field("image_data", vec![])
        .field("category", vec![])
        .build();
    match form.handle_request(request, &state.csrf) {
        Ok(FormStatus::Bound) if form.is_valid() => match parse_new_post(state, &form) {
            Ok(new) => {
                let _ = state.store.update_post(id, &new);
                Ok(redirect(&format!("/#post-{id}")))
            }
            Err(msg) => {
                let categories = state.store.categories();
                let form_html = build_edit_form(&state.csrf, &categories, &post)?;
                Ok(html_response(
                    400,
                    edit_post_page(id, &form_html, Some(msg)),
                ))
            }
        },
        _ => Ok(redirect(&format!("/admin/posts/{id}/edit"))),
    }
}

fn handle_post_delete(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    if !is_admin(request, &state.admin_token) {
        let login = build_login_form(&state.csrf)?;
        return Ok(html_response(
            401,
            admin_login_page(&login, Some("Sign in first.")),
        ));
    }
    let id = route_post_id(request)?;
    let mut form = Form::builder(format!("delete-{id}")).build();
    if form.handle_request(request, &state.csrf).is_err() {
        return Ok(redirect("/"));
    }
    let _ = state.store.delete_post(id);
    Ok(redirect("/"))
}

fn seed_if_empty(store: &FeedStore) {
    if !store.posts().is_empty() {
        return;
    }
    let _ = store.add_post(NewPost {
        body: "<p>Sample clip on the wall.</p>".into(),
        embed_url: Some("https://www.youtube.com/watch?v=MrQ41qf0Rqs".into()),
        image_data: None,
        category: "Ideas".into(),
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind = std::env::var("MYFEED_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let csrf_secret =
        std::env::var("MYFEED_CSRF_SECRET").unwrap_or_else(|_| DEFAULT_CSRF.to_owned());
    let admin_token =
        std::env::var("MYFEED_ADMIN_TOKEN").unwrap_or_else(|_| DEFAULT_ADMIN.to_owned());

    let db_path = std::env::var("MYFEED_DB").unwrap_or_else(|_| ".myfeed.sqlite".to_owned());
    let store = FeedStore::open(db_path);
    seed_if_empty(&store);
    let state = Arc::new(AppState {
        store,
        csrf: HmacCsrfTokenManager::new(csrf_secret.as_bytes()),
        matcher: UrlMatcher::new(routes()?),
        admin_token: admin_token.clone(),
    });

    let state_for_handler = Arc::clone(&state);
    let async_kernel = AsyncHttpKernel::from_sync(move |request: &mut Request| {
        handle(state_for_handler.as_ref(), request)
    });

    println!("MyFeed listening on http://{bind}/");
    println!("Admin: open /admin and sign in with token `{admin_token}`");

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(serenade_http_actix::listen(bind, async_kernel))?;
    Ok(())
}

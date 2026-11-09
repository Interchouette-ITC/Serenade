use std::time::{Duration, Instant};

use serenade_form::{FieldConfig, Form, FormStatus, TextareaOptions};
use serenade_http::{HttpError, Request, Response};
use serenade_profiler::{PROFILER_TOKEN_ATTRIBUTE, QueryEvent, record_query, record_view};

use crate::forms::{
    build_admin_post_actions, build_comment_form, build_like_form, build_post_form, not_blank,
};
use crate::html::{FeedView, feed_page, html_response, redirect};
use crate::state::{AppState, is_admin, ui_for};

pub fn feed_with_flash(
    state: &AppState,
    request: &Request,
    flash: Option<&str>,
    is_err: bool,
    composer_open: bool,
) -> Result<Response, HttpError> {
    let ui = ui_for(state, request);
    let started = Instant::now();
    let categories = state.store.categories();
    let posts = state.store.posts();
    if let Some(token) = request.attributes().get::<String>(PROFILER_TOKEN_ATTRIBUTE) {
        record_query(
            &state.profiler,
            token,
            QueryEvent::new(
                "SELECT posts + categories (feed page)",
                started.elapsed().max(Duration::from_micros(1)),
            ),
        );
    }
    let post_form = build_post_form(&state.csrf, state.matcher.collection(), &categories)?;
    let mut comment_forms = Vec::new();
    let mut like_forms = Vec::new();
    let mut admin_forms = Vec::new();
    let admin = is_admin(request, &state.admin_token);
    for post in &posts {
        comment_forms.push((
            post.id,
            build_comment_form(&state.csrf, state.matcher.collection(), post.id)?,
        ));
        like_forms.push((
            post.id,
            build_like_form(&state.csrf, state.matcher.collection(), post.id)?,
        ));
        if admin {
            admin_forms.push((
                post.id,
                build_admin_post_actions(&state.csrf, state.matcher.collection(), post.id)?,
            ));
        }
    }
    let view_started = Instant::now();
    let body = feed_page(&FeedView {
        ui: &ui,
        store: &state.store,
        post_form_html: &post_form,
        comment_forms: &comment_forms,
        like_forms: &like_forms,
        admin_forms: &admin_forms,
        flash,
        flash_err: is_err,
        composer_open,
        routes: state.matcher.collection(),
    });
    if let Some(token) = request.attributes().get::<String>(PROFILER_TOKEN_ATTRIBUTE) {
        record_view(
            &state.profiler,
            token,
            "feed",
            view_started.elapsed().max(Duration::from_micros(1)),
        );
    }
    Ok(html_response(if is_err { 400 } else { 200 }, body))
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

pub fn handle_comment_create(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let post_id = request
        .attributes()
        .get::<String>("id")
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or_else(|| HttpError::bad_request("bad post id"))?;
    let form_name = format!("comment-{post_id}");
    let mut form = Form::builder(form_name)
        .field_config(
            "body",
            FieldConfig::Textarea(TextareaOptions {
                placeholder: None,
                rows: Some(2),
            }),
            not_blank(),
        )
        .build();
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

pub fn handle_like(state: &AppState, request: &Request) -> Result<Response, HttpError> {
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

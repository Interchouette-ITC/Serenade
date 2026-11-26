use std::time::{Duration, Instant};

use serenade_form::{FieldConfig, Form, FormStatus, TextareaOptions};
use serenade_http::{HttpError, Request, Response};
use serenade_profiler::{PROFILER_TOKEN_ATTRIBUTE, QueryEvent, record_query, record_view};

use crate::forms::{
    build_admin_post_actions, build_comment_form, build_like_form, build_post_form, not_blank,
};
use crate::html::{
    FeedView, comment_window, feed_page, feed_window, html_response, post_card, redirect,
    single_post_page,
};
use crate::i18n::Ui;
use crate::posts::route_post_id;
use crate::state::{AppState, MAX_BODY_CHARS, is_admin, ui_for};
use crate::store::Post;

#[derive(Default)]
struct FeedParams {
    before: Option<u64>,
    after: Option<u64>,
    around: Option<u64>,
    ceiling: Option<u64>,
}

fn id_param(query: Option<&str>, name: &str) -> Result<Option<u64>, HttpError> {
    let mut result = None;
    let mut seen = false;
    for pair in query.unwrap_or("").split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            if pair == name {
                return Err(HttpError::bad_request("missing cursor value"));
            }
            continue;
        };
        if key == name {
            if seen {
                return Err(HttpError::bad_request("duplicate cursor"));
            }
            seen = true;
            if name == "around" && value.is_empty() {
                continue;
            }
            let id = value
                .parse::<u64>()
                .ok()
                .filter(|id| *id > 0 && i64::try_from(*id).is_ok())
                .ok_or_else(|| HttpError::bad_request("invalid cursor"))?;
            result = Some(id);
        }
    }
    Ok(result)
}

impl FeedParams {
    fn parse(request: &Request) -> Result<Self, HttpError> {
        let params = Self {
            before: id_param(request.query(), "before")?,
            after: id_param(request.query(), "after")?,
            around: id_param(request.query(), "around")?,
            ceiling: id_param(request.query(), "ceiling")?,
        };
        if [params.before, params.after, params.around]
            .iter()
            .flatten()
            .count()
            > 1
        {
            return Err(HttpError::bad_request("incompatible cursors"));
        }
        if params.ceiling.is_some_and(|ceiling| {
            [params.before, params.after, params.around]
                .iter()
                .flatten()
                .any(|id| *id > ceiling)
        }) {
            return Err(HttpError::bad_request("cursor exceeds archive ceiling"));
        }
        Ok(params)
    }
}

fn store_error(err: &rusqlite::Error) -> HttpError {
    tracing::error!(%err, "MyFeed database read failed");
    HttpError::failed("Could not load feed data.")
}

fn render_card(
    state: &AppState,
    request: &Request,
    ui: &Ui<'_>,
    post: &Post,
    before: Option<u64>,
    after: Option<u64>,
) -> Result<String, HttpError> {
    let comments = state
        .store
        .comment_batch(post.id, before, after)
        .map_err(|err| store_error(&err))?;
    let comment_form = build_comment_form(&state.csrf, state.matcher.collection(), post.id)?;
    let like_form = build_like_form(&state.csrf, state.matcher.collection(), post.id)?;
    let admin_form = if is_admin(request, &state.admin_token) {
        build_admin_post_actions(&state.csrf, state.matcher.collection(), post.id)?
    } else {
        String::new()
    };
    Ok(post_card(
        ui,
        post,
        &comments,
        &comment_form,
        &like_form,
        &admin_form,
    ))
}

pub fn feed_with_flash(
    state: &AppState,
    request: &Request,
    flash: Option<&str>,
    is_err: bool,
    composer_open: bool,
) -> Result<Response, HttpError> {
    let ui = ui_for(state, request);
    let started = Instant::now();
    let params = FeedParams::parse(request)?;
    let ceiling = params.ceiling.map_or_else(
        || state.store.latest_id().map_err(|err| store_error(&err)),
        Ok,
    )?;
    let mut batch = state
        .store
        .feed_batch(ceiling, params.before, params.after, params.around)
        .map_err(|err| store_error(&err))?;
    if batch.posts.is_empty() && params.around.is_some() {
        batch = state
            .store
            .feed_batch(ceiling, None, params.around, None)
            .map_err(|err| store_error(&err))?;
    }
    let missing_anchor = params
        .around
        .is_some_and(|id| !batch.posts.iter().any(|p| p.id == id));
    let mut cards = String::new();
    for post in &batch.posts {
        cards.push_str(&render_card(state, request, &ui, post, None, None)?);
    }
    let feed_html = feed_window(&ui, &batch, &cards);
    if wants_ajax(request) {
        return Ok(html_response(200, feed_html)
            .with_header("cache-control", "no-store")
            .with_header("x-myfeed-missing", if missing_anchor { "1" } else { "0" }));
    }
    let categories = state.store.categories();
    if let Some(token) = request.attributes().get::<String>(PROFILER_TOKEN_ATTRIBUTE) {
        record_query(
            &state.profiler,
            token,
            QueryEvent::new(
                "SELECT bounded feed + indexed comment previews",
                started.elapsed().max(Duration::from_micros(1)),
            ),
        );
    }
    let post_form = build_post_form(&state.csrf, state.matcher.collection(), &categories)?;
    let admin = is_admin(request, &state.admin_token);
    let missing_message = ui.t("feed_missing");
    let view_started = Instant::now();
    let body = feed_page(&FeedView {
        ui: &ui,
        feed_html: &feed_html,
        post_form_html: &post_form,
        admin,
        flash: flash.or(if missing_anchor {
            Some(&missing_message)
        } else {
            None
        }),
        flash_err: is_err,
        composer_open,
        routes: state.matcher.collection(),
        footer_html: state.footer_html.as_deref(),
        show_admin_link: state.show_admin_link,
    });
    if let Some(token) = request.attributes().get::<String>(PROFILER_TOKEN_ATTRIBUTE) {
        record_view(
            &state.profiler,
            token,
            "feed",
            view_started.elapsed().max(Duration::from_micros(1)),
        );
    }
    Ok(
        html_response(if is_err { 400 } else { 200 }, body)
            .with_header("cache-control", "no-store"),
    )
}

pub fn wants_ajax(request: &Request) -> bool {
    request
        .headers()
        .get("x-myfeed-ajax")
        .is_some_and(|value| value == "1")
}

pub fn handle_latest(state: &AppState) -> Result<Response, HttpError> {
    Ok(ajax_text(
        state
            .store
            .latest_id()
            .map_err(|err| store_error(&err))?
            .to_string(),
    ))
}

pub fn handle_post_get(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let id = route_post_id(request)?;
    let post = state
        .store
        .get_summary(id)
        .map_err(|err| store_error(&err))?
        .ok_or_else(|| HttpError::not_found("post missing"))?;
    let ui = ui_for(state, request);
    let before = id_param(request.query(), "before")?;
    let after = id_param(request.query(), "after")?;
    if before.is_some() && after.is_some() {
        return Err(HttpError::bad_request("incompatible comment cursors"));
    }
    if wants_ajax(request) && request.path().ends_with("/comments") {
        let comments = state
            .store
            .comment_batch(id, before, after)
            .map_err(|err| store_error(&err))?;
        return Ok(html_response(200, comment_window(&ui, id, &comments))
            .with_header("cache-control", "no-store"));
    }
    let card = render_card(state, request, &ui, &post, before, after)?;
    Ok(html_response(
        200,
        single_post_page(&ui, &card, is_admin(request, &state.admin_token)),
    )
    .with_header("cache-control", "no-store"))
}

fn ajax_text(body: impl Into<String>) -> Response {
    Response::new(200)
        .with_header("content-type", "text/plain; charset=utf-8")
        .with_header("cache-control", "no-store")
        .with_body(body.into().into_bytes())
}

pub fn handle_comment_create(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let post_id = route_post_id(request)?;
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
            if body.chars().count() > MAX_BODY_CHARS {
                return Err(HttpError::bad_request("comment exceeds 2000 characters"));
            }
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
        _ if wants_ajax(request) => Err(HttpError::bad_request("Could not send comment.")),
        _ => Ok(redirect("/")),
    }
}

pub fn handle_like(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let post_id = route_post_id(request)?;
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
    let likes = state
        .store
        .like_post(post_id)
        .ok_or_else(|| HttpError::not_found("post missing"))?;
    if wants_ajax(request) {
        return Ok(ajax_text(likes.to_string()));
    }
    Ok(redirect(&format!("/posts/{post_id}")))
}

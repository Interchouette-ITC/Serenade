//! HTML page builders (user plain text via escape; post bodies via ammonia).

use std::fmt::Write as _;

use serenade_http::RouteCollection;
use serenade_view::{asset, escape_attr, escape_html, partial, path};

use crate::embed::{article_html, embed_html};
use crate::emoji::picker_html;
use crate::i18n::Ui;
use crate::preview::LinkPreview;
use crate::store::{Comment, CommentBatch, FeedBatch, Post};

const BOOTSTRAP_CSS: &str =
    "https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css";
const BOOTSTRAP_JS: &str =
    "https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/js/bootstrap.bundle.min.js";
const QUILL_CSS: &str = "https://cdn.jsdelivr.net/npm/quill@2.0.3/dist/quill.snow.css";
const QUILL_JS: &str = "https://cdn.jsdelivr.net/npm/quill@2.0.3/dist/quill.js";

/// Full HTML document with Bootstrap, Quill, and Clitorine.
#[must_use]
pub fn document(title: &str, body: &str, composer_open: bool, lang: &str) -> String {
    let open_flag = if composer_open { "1" } else { "0" };
    let css = asset("myfeed.css?v=10").unwrap_or_else(|_| "/assets/myfeed.css?v=10".into());
    let js = asset("clitorine.js?v=9").unwrap_or_else(|_| "/assets/clitorine.js?v=9".into());
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<link rel="icon" type="image/x-icon" href="/favicon.ico" />
<title>{title}</title>
<link rel="stylesheet" href="{BOOTSTRAP_CSS}" />
<link rel="stylesheet" href="{QUILL_CSS}" />
<link rel="stylesheet" href="{css}" />
</head>
<body class="myfeed" data-composer-open="{open_flag}">
{body}
<script src="{BOOTSTRAP_JS}"></script>
<script src="{QUILL_JS}"></script>
<script src="{js}"></script>
</body>
</html>"#,
        lang = escape_attr(lang),
        title = escape_html(title),
        css = escape_attr(&css),
        js = escape_attr(&js),
    )
}

fn locale_switcher(ui: &Ui<'_>, routes: &RouteCollection) -> String {
    partial(|| {
        let active = ui.locale().language();
        let en_class = if active == "en" {
            "btn btn-sm btn-primary"
        } else {
            "btn btn-sm btn-outline-secondary"
        };
        let fr_class = if active == "fr" {
            "btn btn-sm btn-primary"
        } else {
            "btn btn-sm btn-outline-secondary"
        };
        let en_href =
            path(routes, "locale_set", &[("code", "en")]).unwrap_or_else(|_| "/locale/en".into());
        let fr_href =
            path(routes, "locale_set", &[("code", "fr")]).unwrap_or_else(|_| "/locale/fr".into());
        format!(
            r#"<span class="btn-group" role="group" aria-label="Language">
  <a class="{en_class}" href="{en_href}">{en}</a>
  <a class="{fr_class}" href="{fr_href}">{fr}</a>
</span>"#,
            en_href = escape_attr(&en_href),
            fr_href = escape_attr(&fr_href),
            en = escape_html(&ui.t("lang_en")),
            fr = escape_html(&ui.t("lang_fr")),
        )
    })
}

fn public_admin_link(ui: &Ui<'_>, href: &str, show: bool) -> String {
    if !show {
        return String::new();
    }
    format!(
        r#"<a class="admin-link btn btn-outline-primary btn-sm" href="{}">{}</a>"#,
        escape_attr(href),
        escape_html(&ui.t("nav_admin")),
    )
}

fn public_tagline(ui: &Ui<'_>) -> String {
    escape_html(&ui.t("tagline"))
        .replace("Serenade", r#"<a href="https://serenade.rs/">Serenade</a>"#)
}

/// Inputs for the public feed page.
pub struct FeedView<'a> {
    /// Translated chrome.
    pub ui: &'a Ui<'a>,
    /// Bounded, already-rendered feed window.
    pub feed_html: &'a str,
    /// Composer HTML.
    pub post_form_html: &'a str,
    pub admin: bool,
    /// Optional flash message and whether it is an error.
    pub flash: Option<(&'a str, bool)>,
    /// Open the composer collapse.
    pub composer_open: bool,
    /// Named routes for `path()` helpers.
    pub routes: &'a RouteCollection,
    pub footer_html: Option<&'a str>,
    pub show_admin_link: bool,
}

/// Public feed page.
#[must_use]
pub fn feed_page(view: &FeedView<'_>) -> String {
    let ui = view.ui;
    let flash_html = view.flash.map_or(String::new(), |(msg, is_err)| {
        let class = if is_err {
            "alert alert-danger"
        } else {
            "alert alert-success"
        };
        format!(
            r#"<div class="{class}" role="alert">{msg}</div>"#,
            msg = escape_html(msg)
        )
    });
    let search_href = path(view.routes, "search", &[]).unwrap_or_else(|_| "/search".into());
    let admin_href = path(view.routes, "admin", &[]).unwrap_or_else(|_| "/admin".into());
    let body = format!(
        r#"
<div class="myfeed-shell">
  <header class="myfeed-public-header d-flex flex-wrap align-items-start gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">MyFeed</h1>
      <p class="myfeed-tag">{tagline}</p>
    </div>
    <nav class="myfeed-nav d-flex align-items-center gap-3 pt-2">
      {langs}
      <a class="link-secondary" href="{search_href}">{nav_search}</a>
      {admin_link}
    </nav>
  </header>
  <main>
    {flash_html}
    <section class="card post-card mb-4">
      <div class="card-body p-4">
        <h2 class="h5 mb-2">{composer_title}</h2>
        <p class="text-secondary small mb-3">{composer_help}</p>
        <label class="visually-hidden" for="composer-trigger">{composer_trigger}</label>
        <input id="composer-trigger" type="text" class="form-control form-control-lg composer-trigger mb-2" placeholder="{composer_placeholder}" readonly autocomplete="off" />
        <div class="collapse" id="composer-panel">
          {post_form_html}
        </div>
      </div>
    </section>
    <div id="feed-controller" data-loading="{loading}" data-error="{load_error}"
      data-blocked="{blocked}" data-missing="{missing}">
      <a href="/" id="feed-new-posts" class="btn btn-primary mb-3" hidden>{new_posts}</a>
      <p id="feed-status" role="status" aria-live="polite"></p>
      {feed_html}
    </div>
  </main>
  <footer class="border-top pt-3 mt-4">
    <p class="mb-0 text-center text-secondary small">
      {footer}
    </p>
  </footer>
</div>
{delete_modal}
{about}
{image_modal}
"#,
        tagline = public_tagline(ui),
        langs = locale_switcher(ui, view.routes),
        search_href = escape_attr(&search_href),
        admin_link = public_admin_link(ui, &admin_href, view.show_admin_link),
        nav_search = escape_html(&ui.t("nav_search")),
        composer_title = escape_html(&ui.t("composer_title")),
        composer_help = escape_html(&ui.t("composer_help")),
        composer_trigger = escape_html(&ui.t("composer_trigger")),
        composer_placeholder = escape_attr(&ui.t("composer_placeholder")),
        post_form_html = view.post_form_html,
        footer = view.footer_html.map_or_else(
            || format!(
                r##"<a href="#about-myfeed" class="text-secondary" data-bs-toggle="modal" data-bs-target="#about-myfeed">{}</a>
      <span class="mx-1">·</span>
      {}
      <span class="mx-1">·</span>
      {}"##,
                escape_html(&ui.t("footer_about")),
                escape_html(&ui.t("footer_stack")),
                escape_html(&ui.t("footer_scroll")),
            ),
            str::to_owned,
        ),
        delete_modal = if view.admin {
            delete_post_modal(ui)
        } else {
            String::new()
        },
        feed_html = view.feed_html,
        loading = escape_attr(&ui.t("feed_loading")),
        load_error = escape_attr(&ui.t("feed_error")),
        blocked = escape_attr(&ui.t("feed_blocked")),
        missing = escape_attr(&ui.t("feed_missing")),
        new_posts = escape_html(&ui.t("feed_new_posts")),
        about = if view.footer_html.is_some() {
            String::new()
        } else {
            about_modal(ui)
        },
        image_modal = image_modal(ui),
    );
    document("MyFeed", &body, view.composer_open, ui.locale().language())
}

/// One escaped search hit row (href + snippet already prepared by the caller).
pub struct SearchHitView<'a> {
    /// Anchor to the post on the feed.
    pub href: &'a str,
    /// Plain-text snippet.
    pub snippet: &'a str,
    /// Category label.
    pub category: &'a str,
}

/// Public search page (`GET /search?q=`).
#[must_use]
pub fn search_page(
    ui: &Ui<'_>,
    routes: &RouteCollection,
    query: &str,
    hits: &[SearchHitView<'_>],
    show_admin_link: bool,
) -> String {
    let feed_href = path(routes, "feed", &[]).unwrap_or_else(|_| "/".into());
    let search_href = path(routes, "search", &[]).unwrap_or_else(|_| "/search".into());
    let admin_href = path(routes, "admin", &[]).unwrap_or_else(|_| "/admin".into());
    let results_html = if query.trim().is_empty() {
        format!(
            r#"<p class="text-secondary mb-0">{msg}</p>"#,
            msg = escape_html(&ui.t("search_empty_query")),
        )
    } else if hits.is_empty() {
        format!(
            r#"<p class="text-secondary mb-0">{msg}</p>"#,
            msg = escape_html(&ui.t("search_no_results")),
        )
    } else {
        let mut rows = String::new();
        let count = i64::try_from(hits.len()).unwrap_or(i64::MAX);
        let _ = write!(
            rows,
            r#"<p class="text-secondary small mb-3">{summary}</p><ul class="list-unstyled d-flex flex-column gap-3">"#,
            summary = escape_html(&ui.tn("search_results", count)),
        );
        for hit in hits {
            let _ = write!(
                rows,
                r#"<li class="card post-card"><div class="card-body p-3">
  <a class="stretched-link text-decoration-none" href="{href}"><span class="fw-semibold">{snippet}</span></a>
  <div class="text-secondary small mt-1">{category}</div>
</div></li>"#,
                href = escape_attr(hit.href),
                snippet = escape_html(hit.snippet),
                category = escape_html(hit.category),
            );
        }
        rows.push_str("</ul>");
        rows
    };
    let body = format!(
        r#"
<div class="myfeed-shell">
  <header class="myfeed-public-header d-flex flex-wrap align-items-start gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">MyFeed</h1>
      <p class="myfeed-tag">{tagline}</p>
    </div>
    <nav class="myfeed-nav d-flex align-items-center gap-3 pt-2">
      {langs}
      <a class="link-secondary" href="{feed_href}">{nav_feed}</a>
      {admin_link}
    </nav>
  </header>
  <main>
    <section class="card post-card mb-4">
      <div class="card-body p-4">
        <h2 class="h5 mb-2">{search_title}</h2>
        <p class="text-secondary small mb-3">{search_help}</p>
        <form class="row g-2 align-items-end" method="get" action="{search_action}">
          <div class="col-sm">
            <label class="form-label" for="search-q">{search_title}</label>
            <input id="search-q" class="form-control form-control-lg" type="search" name="q" value="{query}" placeholder="{search_placeholder}" autocomplete="off" />
          </div>
          <div class="col-sm-auto">
            <button class="btn btn-primary btn-lg w-100" type="submit">{search_submit}</button>
          </div>
        </form>
      </div>
    </section>
    <section aria-live="polite">
      {results_html}
    </section>
  </main>
</div>
"#,
        tagline = public_tagline(ui),
        langs = locale_switcher(ui, routes),
        feed_href = escape_attr(&feed_href),
        admin_link = public_admin_link(ui, &admin_href, show_admin_link),
        nav_feed = escape_html(&ui.t("nav_feed")),
        search_title = escape_html(&ui.t("search_title")),
        search_help = escape_html(&ui.t("search_help")),
        search_action = escape_attr(&search_href),
        query = escape_attr(query),
        search_placeholder = escape_attr(&ui.t("search_placeholder")),
        search_submit = escape_html(&ui.t("search_submit")),
    );
    document(
        &format!("MyFeed · {}", ui.t("nav_search")),
        &body,
        false,
        ui.locale().language(),
    )
}

pub fn feed_window(ui: &Ui<'_>, batch: &FeedBatch, cards: &str) -> String {
    let newest = batch.posts.first().map_or(0, |post| post.id);
    let oldest = batch.posts.last().map_or(0, |post| post.id);
    let link = |direction: &str, id: u64, available: bool, label: &str| {
        format!(
            r#"<a data-feed-load="{direction}" class="btn btn-outline-primary my-3"
            href="/?{param}={id}&amp;ceiling={ceiling}" {hidden}>{label}</a>"#,
            param = if direction == "older" {
                "before"
            } else {
                "after"
            },
            ceiling = batch.ceiling,
            hidden = if available { "" } else { "hidden" },
            label = escape_html(&ui.t(label)),
        )
    };
    format!(
        r#"<section id="feed-window" data-ceiling="{ceiling}" data-newest="{newest}"
        data-oldest="{oldest}" data-has-older="{has_older}" data-has-newer="{has_newer}">
        <div data-feed-spacer="top" aria-hidden="true"></div>
        {newer}
        <section id="wall" class="d-flex flex-column gap-4">{cards}{empty}</section>
        {older}
        <div data-feed-spacer="bottom" aria-hidden="true"></div>
        <p data-feed-end {end_hidden} class="text-secondary">{end}</p>
        </section>"#,
        ceiling = batch.ceiling,
        has_older = batch.has_older,
        has_newer = batch.has_newer,
        newer = link("newer", newest, batch.has_newer, "feed_newer"),
        older = link("older", oldest, batch.has_older, "feed_more"),
        empty = if batch.posts.is_empty() {
            escape_html(&ui.t("empty_wall"))
        } else {
            String::new()
        },
        end_hidden = if batch.has_older { "hidden" } else { "" },
        end = escape_html(&ui.t("feed_end")),
    )
}

fn delete_post_modal(ui: &Ui<'_>) -> String {
    format!(
        r#"<div class="modal fade" id="delete-post-modal" tabindex="-1" aria-labelledby="delete-post-modal-label" aria-hidden="true">
  <div class="modal-dialog modal-dialog-centered modal-sm">
    <div class="modal-content">
      <div class="modal-header">
        <h2 class="modal-title fs-6" id="delete-post-modal-label">{title}</h2>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="{close}"></button>
      </div>
      <div class="modal-body small text-secondary">
        {body}
      </div>
      <div class="modal-footer">
        <button type="button" class="btn btn-outline-secondary btn-sm" data-bs-dismiss="modal">{cancel}</button>
        <form id="delete-post-confirm-form" method="POST" action="">
          <span id="delete-post-csrf"></span>
          <button type="submit" class="btn btn-danger btn-sm">{delete}</button>
        </form>
      </div>
    </div>
  </div>
</div>"#,
        title = escape_html(&ui.t("delete_title")),
        close = escape_attr(&ui.t("close")),
        body = escape_html(&ui.t("delete_body")),
        cancel = escape_html(&ui.t("cancel")),
        delete = escape_html(&ui.t("delete")),
    )
}

fn about_modal(ui: &Ui<'_>) -> String {
    format!(
        r#"<div class="modal fade" id="about-myfeed" tabindex="-1" aria-labelledby="about-myfeed-label" aria-hidden="true">
  <div class="modal-dialog modal-dialog-centered">
    <div class="modal-content">
      <div class="modal-header">
        <h2 class="modal-title fs-5" id="about-myfeed-label">{title}</h2>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="{close}"></button>
      </div>
      <div class="modal-body">
        <p>{body}</p>
        <p class="mb-0">{thanks}</p>
      </div>
      <div class="modal-footer">
        <button type="button" class="btn btn-primary" data-bs-dismiss="modal">{close_btn}</button>
      </div>
    </div>
  </div>
</div>"#,
        title = escape_html(&ui.t("about_title")),
        close = escape_attr(&ui.t("close")),
        body = escape_html(&ui.t("about_body")),
        thanks = escape_html(&ui.t("about_thanks")),
        close_btn = escape_html(&ui.t("close")),
    )
}

fn image_modal(ui: &Ui<'_>) -> String {
    format!(
        r#"<div class="modal fade" id="image-modal" tabindex="-1" aria-label="{title}" aria-hidden="true">
  <div class="modal-dialog modal-dialog-centered image-modal-dialog">
    <div class="modal-content image-modal-content">
      <div class="modal-header border-0 py-2">
        <button type="button" class="btn-close ms-auto" data-bs-dismiss="modal" aria-label="{close}"></button>
      </div>
      <div class="modal-body image-modal-body"><img alt="" /></div>
    </div>
  </div>
</div>"#,
        title = escape_attr(&ui.t("image_view")),
        close = escape_attr(&ui.t("close")),
    )
}

pub fn post_card(
    ui: &Ui<'_>,
    post: &Post,
    comments: &CommentBatch,
    comment_form: &str,
    like_form: &str,
    admin_actions: &str,
    preview: Option<&LinkPreview>,
) -> String {
    let embed = post
        .embed_url
        .as_deref()
        .and_then(|url| embed_html(url).or_else(|| article_html(url, preview)))
        .unwrap_or_default();
    let image = if post.has_image {
        format!(
            r#"<div class="feed-image embed-media my-2"><a href="/posts/{id}/image" data-image-preview aria-label="{label}"><img class="img-fluid rounded" src="/posts/{id}/image" alt="posted image" loading="lazy" decoding="async" /></a></div>"#,
            id = post.id,
            label = escape_attr(&ui.t("image_view")),
        )
    } else {
        String::new()
    };
    let comments_html = comment_window(ui, post.id, comments);
    let when = if post.created_at.is_empty() {
        String::new()
    } else {
        format!(" · {}", escape_html(&post.created_at))
    };
    format!(
        r#"
<article class="card post-card" id="post-{id}">
  <div class="card-body p-4">
    <div class="d-flex justify-content-between align-items-start gap-2 mb-2">
      <p class="text-secondary small mb-0"><a href="/posts/{id}">Post #{id}</a> · {category}{when}</p>
      <div class="d-flex flex-wrap align-items-center gap-2">
        <span class="badge text-bg-light border" data-like-count>{likes}</span>
        {like_form}
        {admin_actions}
      </div>
    </div>
    <div class="post-body mb-2">{body}</div>
    {image}
    {embed}
    <div class="border-top pt-3 mt-3">
      <h3 class="h6">{comments_title}</h3>
      <div data-comment-status role="status" aria-live="polite"></div>
      {comments_html}
      {comment_form}
    </div>
  </div>
</article>
"#,
        id = post.id,
        category = escape_html(&post.category),
        likes = escape_html(&ui.tn("likes", i64::try_from(post.likes).unwrap_or(0))),
        comments_title = escape_html(&ui.t("comments")),
        body = post.body,
    )
}

pub fn comment_window(ui: &Ui<'_>, post_id: u64, batch: &CommentBatch) -> String {
    let mut html = String::from("<div data-comment-window>");
    for comment in &batch.comments {
        let _ = write!(
            html,
            r#"<p class="mb-2 small" data-comment-id="{id}">{body}</p>"#,
            id = comment.id,
            body = escape_html(&comment.body),
        );
    }
    if batch.comments.is_empty() {
        let _ = write!(
            html,
            r#"<p class="text-secondary small">{}</p>"#,
            escape_html(&ui.t("no_comments"))
        );
    }
    for (parameter, available, id, label) in [
        (
            "before",
            batch.has_older,
            batch.comments.first().map_or(0, |c| c.id),
            "comments_previous",
        ),
        (
            "after",
            batch.has_newer,
            batch.comments.last().map_or(0, |c| c.id),
            "comments_more",
        ),
    ] {
        if available {
            let _ = write!(
                html,
                r#"<a data-comments-load href="/posts/{post_id}/comments?{parameter}={id}" class="btn btn-sm btn-outline-secondary me-2">{label}</a>"#,
                label = escape_html(&ui.t(label)),
            );
        }
    }
    html.push_str("</div>");
    html
}

pub fn single_post_page(ui: &Ui<'_>, card: &str, admin: bool) -> String {
    let body = format!(
        r#"<main class="myfeed-shell" data-feed-labels data-loading="{loading}" data-error="{error}">
        <nav class="mb-3"><a href="/">{feed}</a></nav>
        {card}</main>{modal}{image_modal}"#,
        feed = escape_html(&ui.t("nav_feed")),
        loading = escape_attr(&ui.t("feed_loading")),
        error = escape_attr(&ui.t("feed_error")),
        modal = if admin {
            delete_post_modal(ui)
        } else {
            String::new()
        },
        image_modal = image_modal(ui),
    );
    document("MyFeed", &body, false, ui.locale().language())
}

/// Admin page with logout, comment queue, and category manager.
#[must_use]
pub fn admin_page_with_logout(
    ui: &Ui<'_>,
    routes: &RouteCollection,
    pending: &[Comment],
    forms: &[(u64, String, String)],
    logout_form: &str,
    categories_html: &str,
    notice: Option<&str>,
) -> String {
    let notice_html = notice.map_or(String::new(), |notice| {
        format!(
            r#"<div class="alert alert-success" role="alert">{notice}</div>"#,
            notice = escape_html(notice)
        )
    });
    let mut rows = String::new();
    if pending.is_empty() {
        let _ = write!(
            rows,
            r#"<p class="text-secondary mb-0">{msg}</p>"#,
            msg = escape_html(&ui.t("queue_empty"))
        );
    }
    for comment in pending {
        let (approve, reject) = forms
            .iter()
            .find(|(id, _, _)| *id == comment.id)
            .map_or(("", ""), |(_, approve, reject)| {
                (approve.as_str(), reject.as_str())
            });
        let _ = write!(
            rows,
            r#"
<div class="card post-card mb-3">
  <div class="card-body">
    <p class="text-secondary small">Comment #{id} on post #{post}</p>
    <p>{body}</p>
    <div class="d-flex gap-2">{approve}{reject}</div>
  </div>
</div>
"#,
            id = comment.id,
            post = comment.post_id,
            body = escape_html(&comment.body),
        );
    }
    let feed_href = path(routes, "feed", &[]).unwrap_or_else(|_| "/".into());
    let admin_href = path(routes, "admin", &[]).unwrap_or_else(|_| "/admin".into());
    let body = format!(
        r#"
<div class="myfeed-shell">
  <header class="d-flex flex-wrap align-items-start gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">{admin_title}</h1>
      <p class="myfeed-tag">{admin_tag}</p>
    </div>
    <nav class="myfeed-nav d-flex align-items-center gap-3 pt-2">
      {langs}
      <a class="link-secondary" href="{feed_href}">{nav_feed}</a>
      <a class="admin-link btn btn-primary btn-sm" href="{admin_href}">{nav_admin}</a>
      {logout_form}
    </nav>
  </header>
  <main class="row g-4">
    <div class="col-lg-7">
      <h2 class="h5 mb-3">{pending_title}</h2>
      {notice_html}
      {rows}
    </div>
    <div class="col-lg-5">
      <h2 class="h5 mb-3">{categories_title}</h2>
      {categories_html}
    </div>
  </main>
</div>
"#,
        admin_title = escape_html(&ui.t("admin_title")),
        admin_tag = escape_html(&ui.t("admin_tag")),
        langs = locale_switcher(ui, routes),
        feed_href = escape_attr(&feed_href),
        admin_href = escape_attr(&admin_href),
        nav_feed = escape_html(&ui.t("nav_feed")),
        nav_admin = escape_html(&ui.t("nav_admin")),
        pending_title = escape_html(&ui.t("pending_title")),
        categories_title = escape_html(&ui.t("categories_title")),
    );
    document(&ui.t("admin_title"), &body, false, ui.locale().language())
}

/// Login form when admin cookie is missing.
#[must_use]
pub fn admin_login_page(
    ui: &Ui<'_>,
    routes: &RouteCollection,
    login_form: &str,
    err: Option<&str>,
) -> String {
    let err_html = err.map_or(String::new(), |msg| {
        format!(
            r#"<div class="alert alert-danger" role="alert">{msg}</div>"#,
            msg = escape_html(msg)
        )
    });
    let feed_href = path(routes, "feed", &[]).unwrap_or_else(|_| "/".into());
    let body = format!(
        r#"
<div class="myfeed-shell">
  <header class="d-flex flex-wrap align-items-start gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">{admin_title}</h1>
      <p class="myfeed-tag">{admin_login_tag}</p>
    </div>
    <nav class="myfeed-nav d-flex align-items-center gap-3 pt-2">
      {langs}
      <a class="link-secondary" href="{feed_href}">{nav_feed}</a>
    </nav>
  </header>
  <main class="card post-card">
    <div class="card-body p-4">
      {err_html}
      <p class="text-secondary small">{admin_login_help}</p>
      {login_form}
    </div>
  </main>
</div>
"#,
        admin_title = escape_html(&ui.t("admin_title")),
        admin_login_tag = escape_html(&ui.t("admin_login_tag")),
        langs = locale_switcher(ui, routes),
        feed_href = escape_attr(&feed_href),
        nav_feed = escape_html(&ui.t("nav_feed")),
        admin_login_help = escape_html(&ui.t("admin_login_help")),
    );
    document(&ui.t("admin_title"), &body, false, ui.locale().language())
}

/// Edit an existing post (admin).
#[must_use]
pub fn edit_post_page(
    ui: &Ui<'_>,
    routes: &RouteCollection,
    post_id: u64,
    form_html: &str,
    err: Option<&str>,
) -> String {
    let err_html = err.map_or(String::new(), |msg| {
        format!(
            r#"<div class="alert alert-danger" role="alert">{msg}</div>"#,
            msg = escape_html(msg)
        )
    });
    let feed_href = path(routes, "feed", &[]).unwrap_or_else(|_| "/".into());
    let admin_href = path(routes, "admin", &[]).unwrap_or_else(|_| "/admin".into());
    let body = format!(
        r#"
<div class="myfeed-shell">
  <header class="d-flex flex-wrap align-items-start gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">Edit post #{post_id}</h1>
      <p class="myfeed-tag">Update body, media, or category</p>
    </div>
    <nav class="myfeed-nav d-flex align-items-center gap-3 pt-2">
      <a class="link-secondary" href="{feed_href}">Feed</a>
      <a class="admin-link btn btn-outline-primary btn-sm" href="{admin_href}">Admin</a>
    </nav>
  </header>
  <main class="card post-card">
    <div class="card-body p-4">
      {err_html}
      {form_html}
    </div>
  </main>
</div>
"#
    );
    document(
        &format!("Edit post #{post_id}"),
        &body,
        false,
        ui.locale().language(),
    )
}

/// HTML response helper.
#[must_use]
pub fn html_response(status: u16, body: String) -> serenade_http::Response {
    serenade_http::Response::new(status)
        .with_header("content-type", "text/html; charset=utf-8")
        .with_body(body.into_bytes())
}

/// Static asset response.
#[must_use]
pub fn asset_response(content_type: &str, bytes: &'static [u8]) -> serenade_http::Response {
    serenade_http::Response::new(200)
        .with_header("content-type", content_type)
        .with_header("cache-control", "no-store")
        .with_body(bytes.to_vec())
}

/// Redirect helper (paths are app-controlled).
#[must_use]
pub fn redirect(location: &str) -> serenade_http::Response {
    serenade_http::Response::new(303)
        .with_header("location", location)
        .with_header("content-type", "text/plain; charset=utf-8")
        .with_body(b"redirect".to_vec())
}

/// Redirect with Set-Cookie.
#[must_use]
pub fn redirect_with_cookie(location: &str, cookie: &str) -> serenade_http::Response {
    redirect(location).with_header("set-cookie", cookie)
}

/// Expose picker for the post form builder.
#[must_use]
pub fn emoji_picker() -> String {
    picker_html()
}

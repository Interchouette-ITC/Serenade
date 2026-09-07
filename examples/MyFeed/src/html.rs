//! HTML page builders (user plain text via `escape_html`; post bodies via ammonia).

use std::fmt::Write as _;

use serenade_form::{escape_attr, escape_html};

use crate::embed::embed_html;
use crate::emoji::picker_html;
use crate::store::{Comment, FeedStore, Post};

const BOOTSTRAP_CSS: &str =
    "https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css";
const BOOTSTRAP_JS: &str =
    "https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/js/bootstrap.bundle.min.js";
const QUILL_CSS: &str = "https://cdn.jsdelivr.net/npm/quill@2.0.3/dist/quill.snow.css";
const QUILL_JS: &str = "https://cdn.jsdelivr.net/npm/quill@2.0.3/dist/quill.js";

/// Full HTML document with Bootstrap, Quill, and Clitorine.
#[must_use]
pub fn document(title: &str, body: &str, composer_open: bool) -> String {
    let open_flag = if composer_open { "1" } else { "0" };
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{title}</title>
<link rel="stylesheet" href="{BOOTSTRAP_CSS}" />
<link rel="stylesheet" href="{QUILL_CSS}" />
<link rel="stylesheet" href="/assets/myfeed.css?v=5" />
</head>
<body class="myfeed" data-composer-open="{open_flag}">
{body}
<script src="{BOOTSTRAP_JS}"></script>
<script src="{QUILL_JS}"></script>
<script src="/assets/clitorine.js?v=4"></script>
</body>
</html>"#,
        title = escape_html(title),
    )
}

/// Inputs for the public feed page.
pub struct FeedView<'a> {
    /// Live store (posts / comments).
    pub store: &'a FeedStore,
    /// Composer HTML.
    pub post_form_html: &'a str,
    /// Per-post comment forms.
    pub comment_forms: &'a [(u64, String)],
    /// Per-post like forms.
    pub like_forms: &'a [(u64, String)],
    /// Per-post admin Edit/Delete (empty when visitor).
    pub admin_forms: &'a [(u64, String)],
    /// Optional flash message.
    pub flash: Option<&'a str>,
    /// Flash is an error.
    pub flash_err: bool,
    /// Open the composer collapse.
    pub composer_open: bool,
}

/// Public feed page.
#[must_use]
pub fn feed_page(view: &FeedView<'_>) -> String {
    let posts_html = render_wall(view);
    let flash_html = view.flash.map_or(String::new(), |msg| {
        let class = if view.flash_err {
            "alert alert-danger"
        } else {
            "alert alert-success"
        };
        format!(
            r#"<div class="{class}" role="alert">{msg}</div>"#,
            msg = escape_html(msg)
        )
    });
    let body = format!(
        r##"
<div class="myfeed-shell">
  <header class="d-flex flex-wrap align-items-start gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">MyFeed</h1>
      <p class="myfeed-tag">Open public wall · self-hosted with Serenade · no signup</p>
    </div>
    <nav class="myfeed-nav d-flex align-items-center gap-3 pt-2">
      <a class="link-secondary" href="/">Feed</a>
      <a class="admin-link btn btn-outline-primary btn-sm" href="/admin">Admin</a>
    </nav>
  </header>
  <main>
    {flash_html}
    <section class="card post-card mb-4">
      <div class="card-body p-4">
        <h2 class="h5 mb-2">Post to the feed</h2>
        <p class="text-secondary small mb-3">Write freely. Attach media when you want.</p>
        <label class="visually-hidden" for="composer-trigger">Open composer</label>
        <input id="composer-trigger" type="text" class="form-control form-control-lg composer-trigger mb-2" placeholder="What's happening?" readonly autocomplete="off" />
        <div class="collapse" id="composer-panel">
          {post_form_html}
        </div>
      </div>
    </section>
    <section id="wall" class="d-flex flex-column gap-4">
      {posts_html}
    </section>
  </main>
  <footer class="border-top pt-3 mt-4">
    <p class="mb-0 text-center text-secondary small">
      <a href="#about-myfeed" class="text-secondary" data-bs-toggle="modal" data-bs-target="#about-myfeed">About</a>
      <span class="mx-1">·</span>
      Bootstrap + Quill + Serenade Form/CSRF
      <span class="mx-1">·</span>
      scroll the wall
    </p>
  </footer>
</div>
{delete_modal}
{about}
"##,
        post_form_html = view.post_form_html,
        delete_modal = if view.admin_forms.is_empty() {
            ""
        } else {
            DELETE_POST_MODAL
        },
        about = ABOUT_MODAL,
    );
    document("MyFeed", &body, view.composer_open)
}

fn render_wall(view: &FeedView<'_>) -> String {
    let posts = view.store.posts();
    if posts.is_empty() {
        return r#"<p class="text-secondary mb-0">No posts yet. Be the first on the wall.</p>"#
            .to_owned();
    }
    let mut posts_html = String::new();
    for post in &posts {
        let comments = view.store.approved_comments(post.id);
        let comment_form = view
            .comment_forms
            .iter()
            .find(|(id, _)| *id == post.id)
            .map_or("", |(_, html)| html.as_str());
        let like_form = view
            .like_forms
            .iter()
            .find(|(id, _)| *id == post.id)
            .map_or("", |(_, html)| html.as_str());
        let admin_actions = view
            .admin_forms
            .iter()
            .find(|(id, _)| *id == post.id)
            .map_or("", |(_, html)| html.as_str());
        posts_html.push_str(&post_card(
            post,
            &comments,
            comment_form,
            like_form,
            admin_actions,
        ));
    }
    posts_html
}

const DELETE_POST_MODAL: &str = r#"<div class="modal fade" id="delete-post-modal" tabindex="-1" aria-labelledby="delete-post-modal-label" aria-hidden="true">
  <div class="modal-dialog modal-dialog-centered modal-sm">
    <div class="modal-content">
      <div class="modal-header">
        <h2 class="modal-title fs-6" id="delete-post-modal-label">Delete this post?</h2>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
      </div>
      <div class="modal-body small text-secondary">
        This removes the post and its comments. You cannot undo it.
      </div>
      <div class="modal-footer">
        <button type="button" class="btn btn-outline-secondary btn-sm" data-bs-dismiss="modal">Cancel</button>
        <form id="delete-post-confirm-form" method="POST" action="">
          <span id="delete-post-csrf"></span>
          <button type="submit" class="btn btn-danger btn-sm">Delete</button>
        </form>
      </div>
    </div>
  </div>
</div>"#;

const ABOUT_MODAL: &str = r#"<div class="modal fade" id="about-myfeed" tabindex="-1" aria-labelledby="about-myfeed-label" aria-hidden="true">
  <div class="modal-dialog modal-dialog-centered">
    <div class="modal-content">
      <div class="modal-header">
        <h2 class="modal-title fs-5" id="about-myfeed-label">About MyFeed</h2>
        <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
      </div>
      <div class="modal-body">
        <p>MyFeed is a beginner demo for the Serenade framework: an open, self-hosted public wall with forms, CSRF, Bootstrap, Quill, and HTML pages you can run locally.</p>
        <p class="mb-0">Thanks to <a href="https://jarvi3.com/pulse" rel="noopener noreferrer" target="_blank">JARVI3 Pulse</a> for inspiring the feed-first shape of this demo. JARVI3 Pulse is original work - not affiliated with Serenade.</p>
      </div>
      <div class="modal-footer">
        <button type="button" class="btn btn-primary" data-bs-dismiss="modal">Close</button>
      </div>
    </div>
  </div>
</div>"#;

fn post_card(
    post: &Post,
    comments: &[Comment],
    comment_form: &str,
    like_form: &str,
    admin_actions: &str,
) -> String {
    let embed = post
        .embed_url
        .as_deref()
        .and_then(embed_html)
        .unwrap_or_default();
    let image = post.image_data.as_deref().map_or(String::new(), |data| {
        if data.starts_with("data:image/") && data.len() < 280_000 {
            format!(
                r#"<div class="embed-media my-2"><img class="img-fluid rounded" src="{src}" alt="posted image" loading="lazy" /></div>"#,
                src = escape_attr(data)
            )
        } else {
            String::new()
        }
    });
    let mut comments_html = String::new();
    for comment in comments {
        let _ = write!(
            comments_html,
            r#"<p class="mb-2 small">{body}</p>"#,
            body = escape_html(&comment.body)
        );
    }
    if comments_html.is_empty() {
        comments_html
            .push_str(r#"<p class="text-secondary small mb-2">No approved comments yet.</p>"#);
    }
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
      <p class="text-secondary small mb-0">Post #{id} · {category}{when}</p>
      <div class="d-flex flex-wrap align-items-center gap-2">
        <span class="badge text-bg-light border" data-like-count>{likes} likes</span>
        {like_form}
        {admin_actions}
      </div>
    </div>
    <div class="post-body mb-2">{body}</div>
    {image}
    {embed}
    <div class="border-top pt-3 mt-3">
      <h3 class="h6">Comments</h3>
      {comments_html}
      {comment_form}
    </div>
  </div>
</article>
"#,
        id = post.id,
        category = escape_html(&post.category),
        likes = post.likes,
        body = post.body,
    )
}

/// Admin page with logout, comment queue, and category manager.
#[must_use]
pub fn admin_page_with_logout(
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
        rows.push_str(r#"<p class="text-secondary mb-0">Queue empty.</p>"#);
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
    let body = format!(
        r#"
<div class="myfeed-shell">
  <header class="d-flex flex-wrap align-items-start gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">MyFeed Admin</h1>
      <p class="myfeed-tag">Moderation queue and categories</p>
    </div>
    <nav class="myfeed-nav d-flex align-items-center gap-3 pt-2">
      <a class="link-secondary" href="/">Feed</a>
      <a class="admin-link btn btn-primary btn-sm" href="/admin">Admin</a>
      {logout_form}
    </nav>
  </header>
  <main class="row g-4">
    <div class="col-lg-7">
      <h2 class="h5 mb-3">Pending comments</h2>
      {notice_html}
      {rows}
    </div>
    <div class="col-lg-5">
      <h2 class="h5 mb-3">Categories</h2>
      {categories_html}
    </div>
  </main>
</div>
"#
    );
    document("MyFeed Admin", &body, false)
}

/// Login form when admin cookie is missing.
#[must_use]
pub fn admin_login_page(login_form: &str, err: Option<&str>) -> String {
    let err_html = err.map_or(String::new(), |msg| {
        format!(
            r#"<div class="alert alert-danger" role="alert">{msg}</div>"#,
            msg = escape_html(msg)
        )
    });
    let body = format!(
        r#"
<div class="myfeed-shell">
  <header class="d-flex flex-wrap align-items-start gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">MyFeed Admin</h1>
      <p class="myfeed-tag">Owner sign-in for the moderation queue</p>
    </div>
    <nav class="myfeed-nav pt-2">
      <a class="link-secondary" href="/">Feed</a>
    </nav>
  </header>
  <main class="card post-card">
    <div class="card-body p-4">
      {err_html}
      <p class="text-secondary small">Use the local demo token from the README (cookie session after login).</p>
      {login_form}
    </div>
  </main>
</div>
"#
    );
    document("MyFeed Admin", &body, false)
}

/// Edit an existing post (admin).
#[must_use]
pub fn edit_post_page(post_id: u64, form_html: &str, err: Option<&str>) -> String {
    let err_html = err.map_or(String::new(), |msg| {
        format!(
            r#"<div class="alert alert-danger" role="alert">{msg}</div>"#,
            msg = escape_html(msg)
        )
    });
    let body = format!(
        r#"
<div class="myfeed-shell">
  <header class="d-flex flex-wrap align-items-start gap-3 mb-4 pb-3 border-bottom">
    <div class="me-auto">
      <h1 class="myfeed-brand">Edit post #{post_id}</h1>
      <p class="myfeed-tag">Update body, media, or category</p>
    </div>
    <nav class="myfeed-nav d-flex align-items-center gap-3 pt-2">
      <a class="link-secondary" href="/">Feed</a>
      <a class="admin-link btn btn-outline-primary btn-sm" href="/admin">Admin</a>
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
    document(&format!("Edit post #{post_id}"), &body, false)
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

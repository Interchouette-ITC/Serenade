use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serenade_filesystem::is_dir;
use serenade_finder::Finder;
use serenade_http::{
    AsyncHttpKernel, HttpError, Method, ROUTE_ATTRIBUTE, Request, Response, Route, RouteCollection,
    UrlMatcher,
};
use serenade_notifier::NullTransport;
use serenade_observability::REQUEST;
use serenade_profiler::{
    AsyncProfilerMiddleware, ProfileStore, ProfilerConfig, ProfilerLogLayer, try_handle_profiler,
};
use serenade_search::MemorySearchAdapter;
use serenade_security::HmacCsrfTokenManager;
use serenade_translation::{Locale, LocaleNegotiator, Translator};
use tracing_subscriber::prelude::*;

use crate::admin::{
    handle_admin_get, handle_admin_login, handle_admin_logout, handle_admin_moderation,
};
use crate::admin_crud;
use crate::feed::{
    feed_with_flash, handle_comment_create, handle_latest, handle_like, handle_post_get,
};
use crate::forms::build_login_form;
use crate::html::asset_response;
use crate::posts::{
    handle_post_create, handle_post_delete, handle_post_edit_get, handle_post_edit_post,
    handle_post_image,
};
use crate::search::{handle_search, rebuild_search_index};
use crate::state::{
    AppState, DEFAULT_ADMIN, DEFAULT_BIND, DEFAULT_CSRF, LOCALE_COOKIE, handle_locale_set,
    is_admin, ui_for,
};
use crate::store::{FeedStore, NewPost};

fn routes() -> Result<RouteCollection, HttpError> {
    let mut collection = RouteCollection::new();
    collection.add(Route::with_method("feed", "/", Method::Get))?;
    collection.add(Route::with_method(
        "feed_latest",
        "/feed/latest",
        Method::Get,
    ))?;
    collection.add(Route::with_method("post_show", "/posts/{id}", Method::Get))?;
    collection.add(Route::with_method(
        "post_image",
        "/posts/{id}/image",
        Method::Get,
    ))?;
    collection.add(Route::with_method(
        "comments_show",
        "/posts/{id}/comments",
        Method::Get,
    ))?;
    collection.add(Route::with_method("search", "/search", Method::Get))?;
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
    admin_crud::register_category_admin_routes(&mut collection)?;
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
    collection.add(Route::with_method(
        "asset_favicon",
        "/favicon.png",
        Method::Get,
    ))?;
    collection.add(Route::with_method(
        "asset_favicon_ico",
        "/favicon.ico",
        Method::Get,
    ))?;
    collection.add(Route::with_method(
        "locale_set",
        "/locale/{code}",
        Method::Get,
    ))?;
    Ok(collection)
}

fn handle(state: &AppState, request: &mut Request) -> Result<Response, HttpError> {
    if let Some(response) = try_handle_profiler(&state.profiler, "/_profiler", request) {
        return Ok(response);
    }
    state.matcher.apply(request)?;
    let _locale = state.negotiator.apply(request);
    tracing::info!(target: REQUEST, path = request.path(), "myfeed request");
    let route = request
        .attributes()
        .get::<String>(ROUTE_ATTRIBUTE)
        .map_or("", String::as_str);

    match route {
        "feed" => feed_with_flash(state, request, None, false, false),
        "feed_latest" => handle_latest(state),
        "post_show" | "comments_show" => handle_post_get(state, request),
        "post_image" => handle_post_image(state, request),
        "search" => Ok(handle_search(state, request)),
        "post_create" => handle_post_create(state, request),
        "comment_create" => handle_comment_create(state, request),
        "post_like" => handle_like(state, request),
        "admin" => handle_admin_get(state, request),
        "admin_login" => handle_admin_login(state, request),
        "admin_logout" => Ok(handle_admin_logout(state, request)),
        "admin_approve" => handle_admin_moderation(state, request, true),
        "admin_reject" => handle_admin_moderation(state, request, false),
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
        "asset_favicon" => Ok(asset_response(
            "image/png",
            include_bytes!("../assets/favicon.png"),
        )),
        "asset_favicon_ico" => Ok(asset_response(
            "image/x-icon",
            include_bytes!("../assets/favicon.ico"),
        )),
        "locale_set" => Ok(handle_locale_set(state, request)),
        _ => {
            let ui = ui_for(state, request);
            admin_crud::try_handle(
                route,
                &admin_crud::CategoryAdminCtx {
                    store: &state.store,
                    csrf: &state.csrf,
                    routes: state.matcher.collection(),
                    request,
                    ui: &ui,
                    is_admin: is_admin(request, &state.admin_token),
                },
                build_login_form,
            )?
            .map_or_else(|| Err(HttpError::not_found("no handler")), Ok)
        }
    }
}

fn seed_if_empty(store: &FeedStore) {
    if store.latest_id().expect("check seed posts") != 0 {
        return;
    }
    let _ = store.add_post(NewPost {
        body: "<p>Hello World from MyFeed - Bootstrap JS + Quill + Serenade forms/CSRF.</p>".into(),
        embed_url: Some("https://www.youtube.com/watch?v=kYfNvmF0Bqw".into()),
        image_data: None,
        category: "Ideas".into(),
    });
    let _ = store.add_post(NewPost {
        body: "<h2>Meet Serenade</h2><p>Serenade is a Rust framework for building dependable web applications, with practical tools for routing, forms, security, and server-rendered HTML.</p>".into(),
        embed_url: None,
        image_data: Some(format!(
            "data:image/png;base64,{}",
            BASE64.encode(include_bytes!("../assets/meet-serenade.png"))
        )),
        category: "Work".into(),
    });
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let bind = std::env::var("MYFEED_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let csrf_secret =
        std::env::var("MYFEED_CSRF_SECRET").unwrap_or_else(|_| DEFAULT_CSRF.to_owned());
    let admin_token =
        std::env::var("MYFEED_ADMIN_TOKEN").unwrap_or_else(|_| DEFAULT_ADMIN.to_owned());

    let db_path = std::env::var("MYFEED_DB").unwrap_or_else(|_| ".myfeed.sqlite".to_owned());
    let store = FeedStore::open(db_path);
    seed_if_empty(&store);
    let index = MemorySearchAdapter::new();
    rebuild_search_index(&store, &index)?;
    let translations = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("translations");
    if !is_dir(&translations) {
        return Err(format!("translations dir missing: {}", translations.display()).into());
    }
    let catalogs = Finder::new()
        .in_path(&translations)
        .files()
        .name("*.toml")
        .collect()
        .map_err(|err| format!("find translations: {err}"))?;
    if catalogs.is_empty() {
        return Err("no translation toml files found".into());
    }
    let mut translator = Translator::new(Locale::new("en").expect("en"))
        .with_fallbacks(vec![Locale::new("en").expect("en")]);
    translator
        .load_path(&translations)
        .map_err(|err| format!("load translations: {err}"))?;
    let negotiator = LocaleNegotiator::new(Locale::new("en").expect("en"))
        .with_allowed(vec![
            Locale::new("en").expect("en"),
            Locale::new("fr").expect("fr"),
        ])
        .with_cookie_name(LOCALE_COOKIE);

    let profiler_enabled = std::env::var("MYFEED_PROFILER").map_or(true, |value| {
        value != "0" && !value.eq_ignore_ascii_case("false")
    });
    let profiler_store = Arc::new(ProfileStore::new(50));
    let _ = tracing_subscriber::registry()
        .with(ProfilerLogLayer)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .try_init();

    let state = Arc::new(AppState {
        store,
        index,
        csrf: HmacCsrfTokenManager::new(csrf_secret.as_bytes()),
        matcher: UrlMatcher::new(routes()?),
        admin_token: admin_token.clone(),
        translator,
        negotiator,
        profiler: Arc::clone(&profiler_store),
        notifier: NullTransport::new(),
    });

    let state_for_handler = Arc::clone(&state);
    let mut async_kernel = AsyncHttpKernel::from_sync(move |request: &mut Request| {
        handle(state_for_handler.as_ref(), request)
    });
    async_kernel.push_middleware(serenade_minify::HtmlMinifier);
    if profiler_enabled {
        async_kernel.push_middleware(AsyncProfilerMiddleware::new(
            Arc::clone(&profiler_store),
            ProfilerConfig::enabled(50),
        ));
    }

    println!("demo-html listening on http://{bind}/");
    println!("Admin: open /admin and sign in with token `{admin_token}`");
    if profiler_enabled {
        println!("Profiler: http://{bind}/_profiler (disable with MYFEED_PROFILER=0)");
    }

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(serenade_http_actix::listen(bind, async_kernel))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::MAX_IMAGE_BYTES;

    #[test]
    fn fresh_demo_includes_meet_serenade_without_duplicates() {
        let store = FeedStore::open_memory();
        seed_if_empty(&store);
        let posts = store.posts();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].id, 2);
        assert_eq!(posts[0].category, "Work");
        assert_eq!(
            posts[0].body,
            "<h2>Meet Serenade</h2><p>Serenade is a Rust framework for building dependable web applications, with practical tools for routing, forms, security, and server-rendered HTML.</p>"
        );
        assert!(posts[1].body.contains("Hello World"));

        seed_if_empty(&store);
        assert_eq!(store.posts().len(), 2);
    }

    fn state() -> AppState {
        let locale = Locale::new("en").expect("locale");
        let mut translator = Translator::new(locale.clone());
        translator
            .load_path(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("translations"))
            .expect("translations");
        AppState {
            store: FeedStore::open_memory(),
            index: MemorySearchAdapter::new(),
            csrf: HmacCsrfTokenManager::new(DEFAULT_CSRF.as_bytes()),
            matcher: UrlMatcher::new(routes().expect("routes")),
            admin_token: DEFAULT_ADMIN.into(),
            translator,
            negotiator: LocaleNegotiator::new(locale.clone())
                .with_allowed(vec![locale, Locale::new("fr").expect("fr")])
                .with_cookie_name(LOCALE_COOKIE),
            profiler: Arc::new(ProfileStore::new(50)),
            notifier: NullTransport::new(),
        }
    }

    fn get(state: &AppState, path: &str, query: &str, ajax: bool) -> Result<Response, HttpError> {
        let mut request = Request::new(Method::Get, path).with_query(query);
        if ajax {
            request = request.with_header("x-myfeed-ajax", "1");
        }
        handle(state, &mut request)
    }

    #[test]
    fn initial_html_and_fragments_have_bounded_media_free_cards() {
        let state = state();
        let data = format!(
            "data:image/png;base64,{}",
            BASE64.encode(vec![1; MAX_IMAGE_BYTES])
        );
        for _ in 0..25 {
            state.store.add_post(NewPost {
                body: "<p>image fixture</p>".into(),
                image_data: Some(data.clone()),
                embed_url: None,
                category: "life".into(),
            });
        }
        let response = get(&state, "/", "", false).expect("initial page");
        let html = response.body_str().expect("html");
        assert_eq!(html.matches("<article ").count(), 20);
        assert!(!html.contains("data:image/"));
        assert!(html.contains("/posts/25/image"));
        assert!(html.len() < 150_000, "initial HTML: {} bytes", html.len());
        let fragment = get(&state, "/", "before=6&ceiling=25", true).expect("fragment");
        let html = fragment.body_str().expect("fragment html");
        assert_eq!(html.matches("<article ").count(), 5);
        assert!(!html.contains("<html"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("data:image/"));
        assert!(html.contains("data-has-older=\"false\""));
        let old = get(&state, "/posts/1", "", false).expect("old post");
        assert_eq!(
            old.body_str()
                .expect("post html")
                .matches("<article ")
                .count(),
            1
        );
        let legacy = get(&state, "/", "around=1", true).expect("legacy anchor");
        assert!(
            legacy
                .body_str()
                .expect("legacy html")
                .contains("id=\"post-1\"")
        );
    }

    #[test]
    fn image_routes_preserve_bytes_and_revalidate_after_editing() {
        let state = state();
        seed_if_empty(&state.store);
        let first = get(&state, "/posts/2/image", "", false).expect("image");
        assert_eq!(first.body(), include_bytes!("../assets/meet-serenade.png"));
        assert_eq!(first.headers().get("content-type"), Some("image/png"));
        let etag = first.headers().get("etag").expect("etag");
        let mut request =
            Request::new(Method::Get, "/posts/2/image").with_header("if-none-match", etag);
        assert_eq!(handle(&state, &mut request).expect("cached").status(), 304);
        state.store.update_post(
            2,
            &NewPost {
                body: "edited".into(),
                category: "life".into(),
                embed_url: None,
                image_data: Some("data:image/png;base64,aGVsbG8=".into()),
            },
        );
        let changed = handle(&state, &mut request).expect("edited image");
        assert_eq!(changed.status(), 200);
        assert_eq!(changed.body(), b"hello");
        assert_ne!(changed.headers().get("etag"), Some(etag));
        assert_eq!(
            get(&state, "/posts/1/image", "", false)
                .expect_err("no image")
                .status_code(),
            404
        );
        assert!(state.store.delete_post(2));
        assert_eq!(
            get(&state, "/posts/2/image", "", false)
                .expect_err("deleted")
                .status_code(),
            404
        );
        let bad = state.store.add_post(NewPost {
            body: "bad image fixture".into(),
            category: "life".into(),
            embed_url: None,
            image_data: Some("data:image/png;base64,!!!!".into()),
        });
        assert_eq!(
            get(&state, &format!("/posts/{}/image", bad.id), "", false)
                .expect_err("invalid image")
                .status_code(),
            500
        );
    }

    #[test]
    fn appended_admin_cards_keep_csrf_actions_and_translations() {
        let state = state();
        for _ in 0..25 {
            state.store.add_post(NewPost {
                body: "<p>admin fixture</p>".into(),
                category: "life".into(),
                embed_url: None,
                image_data: None,
            });
        }
        let mut request = Request::new(Method::Get, "/")
            .with_query("before=6&ceiling=25")
            .with_header("x-myfeed-ajax", "1")
            .with_header("authorization", format!("Bearer {DEFAULT_ADMIN}"))
            .with_header("cookie", "_locale=fr");
        let fragment = handle(&state, &mut request).expect("admin fragment");
        let html = fragment.body_str().expect("html");
        assert_eq!(html.matches("data-clitorine-delete ").count(), 5);
        assert!(html.contains("Commentaires"));
        assert!(html.contains("/admin/posts/5/edit"));
        let token = html
            .split("data-delete-token=\"")
            .nth(1)
            .expect("delete token")
            .split('"')
            .next()
            .expect("token");
        let mut delete = Request::new(Method::Post, "/admin/posts/5/delete")
            .with_header("authorization", format!("Bearer {DEFAULT_ADMIN}"))
            .with_header("content-type", "application/x-www-form-urlencoded")
            .with_body(format!("_token={token}"));
        assert_eq!(handle(&state, &mut delete).expect("delete").status(), 303);
        assert!(
            state
                .store
                .get_summary(5)
                .expect("deleted lookup")
                .is_none()
        );
        let mut invalid = Request::new(Method::Post, "/admin/posts/4/delete")
            .with_header("authorization", format!("Bearer {DEFAULT_ADMIN}"))
            .with_header("content-type", "application/x-www-form-urlencoded")
            .with_body("_token=invalid");
        handle(&state, &mut invalid).expect("invalid CSRF redirect");
        assert!(
            state
                .store
                .get_summary(4)
                .expect("protected post")
                .is_some()
        );
        let mut edit = Request::new(Method::Get, "/admin/posts/4/edit")
            .with_header("authorization", format!("Bearer {DEFAULT_ADMIN}"));
        let response = handle(&state, &mut edit).expect("edit page");
        assert!(
            response
                .body_str()
                .expect("html")
                .contains("href=\"/posts/4\"")
        );
        assert_eq!(response.status(), 200);
    }

    #[test]
    fn cursors_and_comment_routes_have_explicit_contracts() {
        let state = state();
        assert!(
            get(&state, "/", "", false)
                .expect("empty")
                .body_str()
                .expect("html")
                .contains("No posts yet")
        );
        for query in [
            "before=0",
            "before=-1",
            "before=abc",
            "before=1&before=2",
            "before=2&after=3",
            "around=3&ceiling=2",
            "before",
            "before=18446744073709551615",
        ] {
            assert_eq!(
                get(&state, "/", query, true)
                    .expect_err(query)
                    .status_code(),
                400
            );
        }

        let post = state.store.add_post(NewPost {
            body: "comments".into(),
            category: "life".into(),
            embed_url: None,
            image_data: None,
        });
        for i in 0..12 {
            let comment = state
                .store
                .add_comment(post.id, format!("approved {i}"))
                .expect("comment");
            assert!(state.store.approve_comment(comment.id));
        }
        state
            .store
            .add_comment(post.id, "PRIVATE PENDING FIXTURE".into())
            .expect("pending");
        let path = format!("/posts/{}/comments", post.id);
        let preview = get(&state, &path, "", true).expect("comments");
        let html = preview.body_str().expect("comments html");
        assert_eq!(html.matches("data-comment-id=").count(), 10);
        assert!(!html.contains("PRIVATE PENDING"));
        let next = get(&state, &path, "after=10", true).expect("next comments");
        assert_eq!(
            next.body_str()
                .expect("html")
                .matches("data-comment-id=")
                .count(),
            2
        );
        assert!(
            get(&state, &path, "after=10", false)
                .expect("no-JS comments")
                .body_str()
                .expect("document")
                .contains("<html")
        );
        let missing = get(&state, "/", "around=100", true).expect("deleted anchor");
        assert_eq!(missing.headers().get("x-myfeed-missing"), Some("1"));
        assert_eq!(
            get(&state, "/feed/latest", "", false)
                .expect("latest")
                .body_str(),
            Some("1")
        );
    }
    #[test]
    fn seed_preserves_complete_meet_serenade_post() {
        let store = FeedStore::open_memory();
        seed_if_empty(&store);
        let post = store.get_post(2).expect("seeded Post #2");
        assert_eq!(
            post.body,
            "<h2>Meet Serenade</h2><p>Serenade is a Rust framework for building dependable web applications, with practical tools for routing, forms, security, and server-rendered HTML.</p>"
        );
        assert_eq!(post.category, "Work");
        assert!(post.embed_url.is_none());
        let image = post.image_data.expect("original attached image");
        let encoded = image
            .strip_prefix("data:image/png;base64,")
            .expect("PNG data URL");
        let decoded = base64::Engine::decode(&BASE64, encoded).expect("valid base64");
        assert_eq!(decoded, include_bytes!("../assets/meet-serenade.png"));
        assert_eq!(decoded.len(), 207_360);

        seed_if_empty(&store);
        assert_eq!(store.posts().len(), 2);
    }
}

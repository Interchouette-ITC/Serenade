use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
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
    AppState, DEFAULT_ADMIN, DEFAULT_BIND, DEFAULT_CSRF, LOCALE_COOKIE, MAX_IMAGE_BYTES,
    handle_locale_set, is_admin, ui_for,
};
use crate::store::{FeedStore, NewPost};

fn routes(admin_path: &str) -> Result<RouteCollection, HttpError> {
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
    collection.add(Route::with_method("admin", admin_path, Method::Get))?;
    collection.add(Route::with_method(
        "admin_login",
        format!("{admin_path}/login"),
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_logout",
        format!("{admin_path}/logout"),
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_approve",
        format!("{admin_path}/comments/{{id}}/approve"),
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_reject",
        format!("{admin_path}/comments/{{id}}/reject"),
        Method::Post,
    ))?;
    admin_crud::register_category_admin_routes(&mut collection, admin_path)?;
    collection.add(Route::with_method(
        "admin_post_edit_get",
        format!("{admin_path}/posts/{{id}}/edit"),
        Method::Get,
    ))?;
    collection.add(Route::with_method(
        "admin_post_edit_post",
        format!("{admin_path}/posts/{{id}}/edit"),
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "admin_post_delete",
        format!("{admin_path}/posts/{{id}}/delete"),
        Method::Post,
    ))?;
    collection.add(Route::with_method(
        "asset_css",
        "/assets/myfeed.css",
        Method::Get,
    ))?;
    collection.add(Route::with_method(
        "asset_theme",
        "/assets/theme/{name}",
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

fn admin_path(value: &str) -> Result<String, String> {
    let segment = value.strip_prefix('/').unwrap_or(value);
    if segment.is_empty()
        || !segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        || ["feed", "posts", "search", "assets", "locale", "_profiler"].contains(&segment)
    {
        return Err("MYFEED_ADMIN_PATH must be a non-reserved single URL segment containing only letters, digits, '-' or '_'".into());
    }
    Ok(format!("/{segment}"))
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
        "feed" => feed_with_flash(state, request, None, false, false)
            .map(|response| preload_theme_image(state, request, response)),
        "feed_latest" => handle_latest(state),
        "post_show" | "comments_show" => handle_post_get(state, request),
        "post_image" => handle_post_image(state, request),
        "search" => Ok(preload_theme_image(
            state,
            request,
            handle_search(state, request),
        )),
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
        "asset_css" => Ok(theme_css_response(state)),
        "asset_theme" => theme_asset_response(state, request),
        "asset_js" => Ok(asset_response(
            "text/javascript; charset=utf-8",
            include_bytes!("../assets/clitorine.min.js"),
        )),
        "asset_favicon" => Ok(asset_response(
            "image/png",
            include_bytes!("../assets/favicon.png"),
        )),
        "asset_favicon_ico" => Ok(favicon_response(state)),
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

fn preload_theme_image(state: &AppState, request: &Request, response: Response) -> Response {
    if crate::feed::wants_ajax(request) || response.status() != 200 {
        return response;
    }
    match &state.theme_preload_image {
        Some(name) => response.with_header(
            "link",
            format!("</assets/theme/{name}>; rel=preload; as=image"),
        ),
        None => response,
    }
}

fn theme_asset_type(name: &str) -> Option<&'static str> {
    let (stem, extension) = name.rsplit_once('.')?;
    if stem.is_empty()
        || !stem
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return None;
    }
    match extension {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "js" => Some("text/javascript; charset=utf-8"),
        _ => None,
    }
}

fn favicon_response(state: &AppState) -> Response {
    let Some(path) = &state.favicon_file else {
        return asset_response("image/x-icon", include_bytes!("../assets/favicon.ico"));
    };
    match std::fs::read(path) {
        Ok(bytes) => Response::new(200)
            .with_header("content-type", "image/x-icon")
            .with_header("cache-control", "no-store")
            .with_body(bytes),
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "MyFeed favicon could not be read");
            Response::text(500, "MyFeed favicon could not be read.")
                .with_header("cache-control", "no-store")
        }
    }
}

fn theme_asset_response(state: &AppState, request: &Request) -> Result<Response, HttpError> {
    let not_found = || HttpError::not_found("theme asset not found");
    let name = request
        .attributes()
        .get::<String>("name")
        .ok_or_else(not_found)?;
    let content_type = theme_asset_type(name).ok_or_else(not_found)?;
    let path = state
        .theme_css
        .as_ref()
        .ok_or_else(not_found)?
        .with_file_name(name);
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => return Err(not_found()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(not_found()),
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "theme asset could not be inspected");
            return Ok(Response::text(500, "Theme asset could not be read.")
                .with_header("cache-control", "no-store"));
        }
    }
    match std::fs::read(&path) {
        Ok(bytes) if content_type.starts_with("image/") => {
            let mut hash = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut hash);
            content_type.hash(&mut hash);
            let etag = format!("\"{:x}\"", hash.finish());
            let response = if request.headers().get("if-none-match") == Some(etag.as_str()) {
                Response::new(304)
            } else {
                Response::new(200)
                    .with_header("content-type", content_type)
                    .with_body(bytes)
            };
            Ok(response
                .with_header("etag", etag)
                .with_header("cache-control", "no-cache"))
        }
        Ok(bytes) => Ok(Response::new(200)
            .with_header("content-type", content_type)
            .with_header("cache-control", "no-store")
            .with_body(bytes)),
        Err(error) => {
            tracing::error!(path = %path.display(), %error, "theme asset could not be read");
            Ok(Response::text(500, "Theme asset could not be read.")
                .with_header("cache-control", "no-store"))
        }
    }
}

fn theme_css_response(state: &AppState) -> Response {
    let mut css = include_bytes!("../assets/myfeed.css").to_vec();
    if let Some(path) = &state.theme_css {
        match std::fs::read(path) {
            Ok(theme) => {
                css.push(b'\n');
                css.extend_from_slice(&theme);
            }
            Err(error) => {
                tracing::error!(
                    path = %path.display(),
                    error = %error,
                    "configured MyFeed theme CSS could not be read"
                );
                return Response::text(500, "Configured MyFeed theme CSS could not be read.")
                    .with_header("cache-control", "no-store");
            }
        }
    }

    Response::new(200)
        .with_header("content-type", "text/css; charset=utf-8")
        .with_header("cache-control", "no-store")
        .with_body(css)
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

fn validate_theme_files(
    theme: Option<&Path>,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let Some(theme) = theme else {
        return Ok(None);
    };
    std::fs::read(theme).map_err(|error| {
        format!(
            "MYFEED_THEME_CSS={} could not be read: {error}",
            theme.display()
        )
    })?;
    let path = theme.with_file_name("favicon.ico");
    if path
        .try_exists()
        .map_err(|error| format!("check favicon {}: {error}", path.display()))?
    {
        std::fs::read(&path)
            .map_err(|error| format!("read favicon {}: {error}", path.display()))?;
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn theme_preload_image(
    theme_css: Option<&Path>,
    configured: Option<String>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if let Some(name) = configured {
        if !theme_asset_type(&name).is_some_and(|kind| kind.starts_with("image/"))
            || theme_css.is_none()
        {
            return Err("MYFEED_THEME_PRELOAD_IMAGE must name a supported theme image and requires MYFEED_THEME_CSS".into());
        }
        return Ok(Some(name));
    }

    let Some(theme_css) = theme_css else {
        return Ok(None);
    };
    let path = theme_css.with_file_name("header.webp");
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(Some("header.webp".into())),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("check theme header {}: {error}", path.display()).into()),
    }
}

fn load_translator() -> Result<Translator, Box<dyn std::error::Error>> {
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
    Ok(translator)
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let bind = std::env::var("MYFEED_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let csrf_secret =
        std::env::var("MYFEED_CSRF_SECRET").unwrap_or_else(|_| DEFAULT_CSRF.to_owned());
    let admin_token =
        std::env::var("MYFEED_ADMIN_TOKEN").unwrap_or_else(|_| DEFAULT_ADMIN.to_owned());
    let admin_path = match std::env::var("MYFEED_ADMIN_PATH") {
        Ok(value) => admin_path(&value)?,
        Err(std::env::VarError::NotPresent) => "/admin".to_owned(),
        Err(error) => return Err(format!("invalid MYFEED_ADMIN_PATH: {error}").into()),
    };
    let theme_css = std::env::var_os("MYFEED_THEME_CSS").map(PathBuf::from);
    let configured_preload_image = match std::env::var("MYFEED_THEME_PRELOAD_IMAGE") {
        Ok(name) => Some(name),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(format!("invalid MYFEED_THEME_PRELOAD_IMAGE: {error}").into()),
    };
    let theme_preload_image = theme_preload_image(theme_css.as_deref(), configured_preload_image)?;
    let footer_html = std::env::var_os("MYFEED_FOOTER_FILE")
        .map(PathBuf::from)
        .map(|path| {
            std::fs::read_to_string(&path).map_err(|error| {
                format!(
                    "MYFEED_FOOTER_FILE={} could not be read: {error}",
                    path.display()
                )
            })
        })
        .transpose()?;
    let favicon_file = validate_theme_files(theme_css.as_deref())?;

    let db_path = std::env::var("MYFEED_DB").unwrap_or_else(|_| ".myfeed.sqlite".to_owned());
    let store = FeedStore::open(db_path);
    seed_if_empty(&store);
    let index = MemorySearchAdapter::new();
    rebuild_search_index(&store, &index)?;
    let translator = load_translator()?;
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
        matcher: UrlMatcher::new(routes(&admin_path)?),
        admin_token,
        translator,
        negotiator,
        profiler: Arc::clone(&profiler_store),
        notifier: NullTransport::new(),
        theme_css,
        theme_preload_image,
        footer_html,
        show_admin_link: admin_path == "/admin",
        favicon_file,
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
    println!("Admin: open {admin_path} and sign in with the configured token");
    if profiler_enabled {
        println!("Profiler: http://{bind}/_profiler (disable with MYFEED_PROFILER=0)");
    }

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(serenade_http_actix::listen_with_payload_limit(
        bind,
        async_kernel,
        2 * MAX_IMAGE_BYTES,
    ))?;
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
            matcher: UrlMatcher::new(routes("/admin").expect("routes")),
            admin_token: DEFAULT_ADMIN.into(),
            translator,
            negotiator: LocaleNegotiator::new(locale.clone())
                .with_allowed(vec![locale, Locale::new("fr").expect("fr")])
                .with_cookie_name(LOCALE_COOKIE),
            profiler: Arc::new(ProfileStore::new(50)),
            notifier: NullTransport::new(),
            theme_css: None,
            theme_preload_image: None,
            footer_html: None,
            show_admin_link: true,
            favicon_file: None,
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
    fn editing_accepts_images_up_to_the_one_mib_limit() {
        use serenade_security::CsrfTokenManager;

        let state = state();
        seed_if_empty(&state.store);
        let token = state.csrf.get_token("edit-2").expect("CSRF token");
        let boundary = "myfeed-test-boundary";
        for size in [600 * 1024, MAX_IMAGE_BYTES, MAX_IMAGE_BYTES + 1] {
            let mut body = format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"_token\"\r\n\r\n{}\r\n\
                 --{boundary}\r\nContent-Disposition: form-data; name=\"body\"\r\n\r\n<p>edited</p>\r\n\
                 --{boundary}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"image.png\"\r\n\
                 Content-Type: image/png\r\n\r\n",
                token.value()
            )
            .into_bytes();
            body.extend(vec![b'a'; size]);
            body.extend(format!("\r\n--{boundary}--\r\n").as_bytes());
            let mut request = Request::new(Method::Post, "/admin/posts/2/edit")
                .with_header("Authorization", format!("Bearer {DEFAULT_ADMIN}"))
                .with_header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .with_body(body);
            let response = handle(&state, &mut request).expect("edit response");
            let expected = if size <= MAX_IMAGE_BYTES {
                "/posts/2"
            } else {
                "/admin/posts/2/edit"
            };
            assert_eq!(response.headers().get("location"), Some(expected));
            let image = get(&state, "/posts/2/image", "", false).expect("stored image");
            assert_eq!(image.body().len(), size.min(MAX_IMAGE_BYTES));
        }
    }

    #[test]
    fn public_navigation_omits_the_current_page() {
        let state = state();
        for locale in ["en", "fr"] {
            for (page, present, absent) in [
                ("/", "href=\"/search\"", "href=\"/\""),
                ("/search", "href=\"/\"", "href=\"/search\""),
            ] {
                let mut request = Request::new(Method::Get, page)
                    .with_header("cookie", format!("_locale={locale}"));
                let response = handle(&state, &mut request).expect("page");
                let html = response.body_str().expect("page HTML");
                let nav = html
                    .split("<nav ")
                    .nth(1)
                    .expect("navigation")
                    .split("</nav>")
                    .next()
                    .expect("navigation end");
                assert!(nav.contains(present), "{page} {locale}");
                assert!(!nav.contains(absent), "{page} {locale}");
                assert!(nav.contains("href=\"/admin\""), "{page} {locale}");
            }
        }
    }

    #[test]
    fn public_tagline_links_to_serenade_in_both_languages() {
        let state = state();
        for (locale, expected) in [
            ("en", "Open public wall · self-hosted with "),
            ("fr", "Mur public ouvert · auto-hébergé avec "),
        ] {
            for page in ["/", "/search"] {
                let mut request = Request::new(Method::Get, page)
                    .with_header("cookie", format!("_locale={locale}"));
                let response = handle(&state, &mut request).expect("page");
                let html = response.body_str().expect("page HTML");
                assert!(
                    html.contains(&format!(
                        r#"{expected}<a href="https://serenade.rs/">Serenade</a>"#
                    )),
                    "{page} {locale}"
                );
            }
        }
    }

    #[test]
    fn custom_admin_path_moves_routes_and_hides_public_links() {
        let mut state = state();
        let prefix = "/private-test";
        state.matcher = UrlMatcher::new(routes(prefix).expect("custom routes"));
        state.show_admin_link = false;
        seed_if_empty(&state.store);

        for page in ["/", "/search"] {
            let response = get(&state, page, "", false).expect("public page");
            let html = response.body_str().expect("public HTML");
            assert!(!html.contains("class=\"admin-link"));
            assert!(!html.contains(prefix));
        }
        assert_eq!(
            get(&state, "/admin", "", false)
                .expect_err("old admin path")
                .status_code(),
            404
        );
        let login = get(&state, prefix, "", false).expect("custom login");
        let html = login.body_str().expect("login HTML");
        assert!(html.contains("action=\"/private-test/login\""));
        let token = html
            .split("name=\"_token\"")
            .nth(1)
            .expect("CSRF field")
            .split("value=\"")
            .nth(1)
            .expect("CSRF value")
            .split('"')
            .next()
            .expect("CSRF token");
        let mut request = Request::new(Method::Post, "/private-test/login")
            .with_header("content-type", "application/x-www-form-urlencoded")
            .with_body(format!("_token={token}&token={DEFAULT_ADMIN}"));
        let response = handle(&state, &mut request).expect("login");
        assert_eq!(response.status(), 303);
        assert_eq!(response.headers().get("location"), Some(prefix));
        let cookie = response.headers().get("set-cookie").expect("login cookie");

        let mut request = Request::new(Method::Get, prefix).with_header("cookie", cookie);
        let dashboard = handle(&state, &mut request).expect("dashboard");
        let html = dashboard.body_str().expect("dashboard HTML");
        assert!(html.contains("/private-test/categories"));
        assert!(html.contains("/private-test/logout"));

        for page in [
            "/private-test/categories",
            "/private-test/categories/new",
            "/private-test/posts/2/edit",
        ] {
            let mut request = Request::new(Method::Get, page).with_header("cookie", cookie);
            let response = handle(&state, &mut request).expect("admin page");
            assert_eq!(response.status(), 200);
            assert!(
                response
                    .body_str()
                    .expect("HTML")
                    .contains("/private-test/")
            );
            assert!(!response.body_str().expect("HTML").contains("/admin/"));
        }
        let mut edit = Request::new(Method::Post, "/private-test/posts/2/edit")
            .with_header("cookie", cookie)
            .with_header("content-type", "application/x-www-form-urlencoded")
            .with_body("_token=invalid");
        assert_eq!(
            handle(&state, &mut edit)
                .expect("invalid edit")
                .headers()
                .get("location"),
            Some("/private-test/posts/2/edit")
        );
    }

    #[test]
    fn admin_path_rejects_reserved_and_invalid_segments() {
        assert_eq!(admin_path("custom-123").expect("valid"), "/custom-123");
        assert_eq!(admin_path("/custom_123").expect("valid"), "/custom_123");
        for value in [
            "",
            "/",
            "posts",
            "search",
            "_profiler",
            "x/y",
            "//x",
            "{id}",
            "a?b",
            "..",
        ] {
            assert!(admin_path(value).is_err(), "{value}");
        }
    }

    #[test]
    fn configured_favicon_serves_the_site_icon() {
        let mut state = state();
        let default = get(&state, "/favicon.ico", "", false).expect("default icon");
        assert_eq!(default.status(), 200);
        assert_eq!(default.body(), include_bytes!("../assets/favicon.ico"));

        let path = std::env::temp_dir().join(format!(
            "serenade-myfeed-favicon-test-{}.ico",
            std::process::id()
        ));
        std::fs::write(&path, b"example icon").expect("write icon");
        state.favicon_file = Some(path.clone());
        let response = get(&state, "/favicon.ico", "", false).expect("custom icon");
        assert_eq!(response.status(), 200);
        assert_eq!(response.body(), b"example icon");
        assert_eq!(response.headers().get("content-type"), Some("image/x-icon"));
        assert_eq!(response.headers().get("cache-control"), Some("no-store"));
        std::fs::remove_file(path).expect("remove icon");
        assert_eq!(
            get(&state, "/favicon.ico", "", false)
                .expect("missing icon response")
                .status(),
            500
        );
    }

    #[test]
    fn custom_footer_preserves_default_and_removes_about() {
        let mut state = state();
        let default = get(&state, "/", "", false).expect("default feed");
        let html = default.body_str().expect("default HTML");
        assert!(html.contains("href=\"#about-myfeed\""));
        assert!(html.contains("id=\"about-myfeed\""));

        state.footer_html =
            Some(r#"Built with <a href="https://serenade.rs/">Serenade</a>"#.into());
        let customized = get(&state, "/", "", false).expect("customized feed");
        let html = customized.body_str().expect("customized HTML");
        assert!(html.contains(r#"Built with <a href="https://serenade.rs/">Serenade</a>"#));
        assert!(!html.contains("about-myfeed"));
        assert!(!html.contains("Bootstrap + Quill"));
    }

    #[test]
    fn configured_theme_css_is_live_and_not_cached() {
        let directory =
            std::env::temp_dir().join(format!("serenade-myfeed-theme-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("create theme test directory");
        let path = directory.join("theme.css");
        std::fs::write(&path, "body.myfeed { color: teal; }").expect("write theme");

        let mut state = state();
        state.theme_css = Some(path.clone());
        let response = get(&state, "/assets/myfeed.css", "", false).expect("theme response");
        let body = response.body_str().expect("CSS body");
        assert!(body.starts_with("/* MyFeed: Bootstrap purple theme */"));
        assert!(body.ends_with("body.myfeed { color: teal; }"));
        assert_eq!(
            response.headers().get("content-type"),
            Some("text/css; charset=utf-8")
        );
        assert_eq!(response.headers().get("cache-control"), Some("no-store"));

        std::fs::write(&path, "body.myfeed { color: olive; }").expect("update theme");
        let refreshed = get(&state, "/assets/myfeed.css", "", false).expect("refreshed theme");
        assert!(
            refreshed
                .body_str()
                .expect("refreshed CSS body")
                .ends_with("body.myfeed { color: olive; }")
        );

        std::fs::remove_file(path).expect("remove theme file");
        std::fs::remove_dir(directory).expect("remove theme test directory");
    }

    #[test]
    fn theme_assets_are_served_only_from_the_configured_theme() {
        let directory = std::env::temp_dir().join(format!(
            "serenade-myfeed-theme-images-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create theme directory");
        let image = directory.join("header.webp");
        std::fs::write(&image, b"example image").expect("write image");
        let mut state = state();
        assert_eq!(
            get(&state, "/assets/theme/header.webp", "", false)
                .expect_err("no theme")
                .status_code(),
            404
        );
        state.theme_css = Some(directory.join("theme.css"));
        let response = get(&state, "/assets/theme/header.webp", "", false).expect("image");
        assert_eq!(response.body(), b"example image");
        assert_eq!(response.headers().get("content-type"), Some("image/webp"));
        assert_eq!(response.headers().get("cache-control"), Some("no-cache"));
        let etag = response
            .headers()
            .get("etag")
            .expect("image etag")
            .to_owned();
        let mut request = Request::new(Method::Get, "/assets/theme/header.webp")
            .with_header("if-none-match", &etag);
        let unchanged = handle(&state, &mut request).expect("unchanged image");
        assert_eq!(unchanged.status(), 304);
        assert_eq!(unchanged.body(), b"");
        std::fs::write(&image, b"changed image").expect("replace image");
        let mut request = Request::new(Method::Get, "/assets/theme/header.webp")
            .with_header("if-none-match", &etag);
        let changed = handle(&state, &mut request).expect("changed image");
        assert_eq!(changed.status(), 200);
        assert_eq!(changed.body(), b"changed image");
        assert_ne!(changed.headers().get("etag"), Some(etag.as_str()));

        let script = directory.join("decor.js");
        std::fs::write(&script, b"example script").expect("write script");
        let response = get(&state, "/assets/theme/decor.js", "", false).expect("script");
        assert_eq!(response.body(), b"example script");
        assert_eq!(
            response.headers().get("content-type"),
            Some("text/javascript; charset=utf-8")
        );
        assert_eq!(response.headers().get("cache-control"), Some("no-store"));

        for path in [
            "/assets/theme/missing.png",
            "/assets/theme/theme.css",
            "/assets/theme/..%2Fheader.webp",
            "/assets/theme/.header.webp",
        ] {
            assert_eq!(
                get(&state, path, "", false)
                    .expect_err("invalid or missing image")
                    .status_code(),
                404,
                "{path}"
            );
        }
        std::fs::remove_file(script).expect("remove script");
        std::fs::remove_file(image).expect("remove image");
        std::fs::remove_dir(directory).expect("remove theme directory");
    }

    #[test]
    fn configured_theme_image_is_preloaded_only_on_full_public_headers() {
        let mut state = state();
        let response = get(&state, "/", "", false).expect("feed");
        assert_eq!(response.headers().get("link"), None);
        state.theme_preload_image = Some("header.webp".into());
        for path in ["/", "/search"] {
            let response = get(&state, path, "", false).expect("public page");
            assert_eq!(
                response.headers().get("link"),
                Some("</assets/theme/header.webp>; rel=preload; as=image")
            );
        }
        let fragment = get(&state, "/", "", true).expect("feed fragment");
        assert_eq!(fragment.headers().get("link"), None);
        let admin = get(&state, "/admin", "", false).expect("admin");
        assert_eq!(admin.headers().get("link"), None);
    }

    #[test]
    fn default_theme_preload_uses_existing_header_unless_overridden() {
        let directory = std::env::temp_dir().join(format!(
            "serenade-myfeed-theme-preload-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create theme directory");
        let theme = directory.join("theme.css");
        assert_eq!(
            theme_preload_image(Some(&theme), None).expect("missing header"),
            None
        );
        assert_eq!(theme_preload_image(None, None).expect("no theme"), None);
        let header = directory.join("header.webp");
        std::fs::write(&header, b"image").expect("write header");
        assert_eq!(
            theme_preload_image(Some(&theme), None)
                .expect("default header")
                .as_deref(),
            Some("header.webp")
        );
        assert_eq!(
            theme_preload_image(Some(&theme), Some("other.png".into()))
                .expect("custom header")
                .as_deref(),
            Some("other.png")
        );
        assert!(theme_preload_image(Some(&theme), Some("../header.webp".into())).is_err());
        assert!(theme_preload_image(None, Some("header.webp".into())).is_err());
        std::fs::remove_file(header).expect("remove header");
        std::fs::remove_dir(directory).expect("remove theme directory");
    }

    #[test]
    fn missing_configured_theme_returns_an_explicit_error() {
        let mut state = state();
        state.theme_css = Some(
            std::env::temp_dir()
                .join(format!(
                    "serenade-myfeed-theme-missing-{}",
                    std::process::id()
                ))
                .join("missing.css"),
        );

        let response = get(&state, "/assets/myfeed.css", "", false).expect("error response");
        assert_eq!(response.status(), 500);
        assert_eq!(
            response.body_str(),
            Some("Configured MyFeed theme CSS could not be read.")
        );
        assert_eq!(response.headers().get("cache-control"), Some("no-store"));
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
        assert!(html.contains("data-image-preview"));
        assert!(html.contains("id=\"image-modal\""));
        assert!(html.len() < 150_000, "initial HTML: {} bytes", html.len());
        let fragment = get(&state, "/", "before=6&ceiling=25", true).expect("fragment");
        let html = fragment.body_str().expect("fragment html");
        assert_eq!(html.matches("<article ").count(), 5);
        assert!(!html.contains("<html"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("data:image/"));
        assert!(html.contains("data-image-preview"));
        assert!(!html.contains("id=\"image-modal\""));
        assert!(html.contains("data-has-older=\"false\""));
        let old = get(&state, "/posts/1", "", false).expect("old post");
        assert!(
            old.body_str()
                .expect("post html")
                .contains("id=\"image-modal\"")
        );
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
            "before=",
            "before=1&before=2",
            "around=&around=1",
            "around=1&around=",
            "around=0",
            "around=abc",
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
    fn empty_jump_opens_feed() {
        let state = state();
        seed_if_empty(&state.store);
        let response = get(&state, "/", "around=", false).expect("empty jump");
        assert_eq!(response.status(), 200);
        let html = response.body_str().expect("feed HTML");
        assert!(html.contains("Hello World"));
        assert!(html.contains("Meet Serenade"));
        assert!(html.contains("<iframe"));
        assert!(!html.contains("data-release-media"));
        assert!(!html.contains("id=\"feed-jump\""));

        let fragment = get(&state, "/", "around=", true).expect("empty jump fragment");
        assert_eq!(fragment.status(), 200);
        assert_eq!(fragment.headers().get("x-myfeed-missing"), Some("0"));
        assert!(fragment.body_str().expect("fragment").contains("post-2"));
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

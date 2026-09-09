//! Unit tests for `serenade-view`.

use serenade_http::{Method, Route, RouteCollection};

use crate::{AssetConfig, asset, escape_html, partial, path, version};

#[test]
fn crate_version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn path_wraps_route_generate() {
    let mut routes = RouteCollection::new();
    routes
        .add(Route::with_method("item", "/items/{id}", Method::Get))
        .expect("add");
    assert_eq!(
        path(&routes, "item", &[("id", "9")]).expect("path"),
        "/items/9"
    );
}

#[test]
fn asset_joins_and_rejects_unsafe() {
    assert_eq!(asset("myfeed.css").expect("ok"), "/assets/myfeed.css");
    assert_eq!(
        asset("myfeed.css?v=5").expect("query"),
        "/assets/myfeed.css?v=5"
    );
    assert_eq!(
        AssetConfig::with_base("/static")
            .join("js/app.js")
            .expect("ok"),
        "/static/js/app.js"
    );
    assert_eq!(asset("").expect("base"), "/assets");
    assert!(asset("../secret").is_err());
    assert!(asset("http://evil.example/x").is_err());
    assert!(asset("a//b").is_err());
    assert!(
        AssetConfig::with_base("cdn")
            .join("x")
            .expect("rel")
            .starts_with('/')
    );
    assert_eq!(
        AssetConfig::with_base("").join("x").expect("blank base"),
        "/x"
    );
}

#[test]
fn partial_runs_fragment() {
    let html = partial(|| format!("<span>{}</span>", escape_html("<ok>")));
    assert_eq!(html, "<span>&lt;ok&gt;</span>");
}

use serenade_http::{Request, Response};
use serenade_search::{DocumentIndex, MemorySearchAdapter, SearchDocument, SearchQuery};

use crate::html::{SearchHitView, html_response, search_page};
use crate::sanitize::plain_text;
use crate::state::{AppState, ui_for};
use crate::store::{FeedStore, Post};

fn post_search_document(post: &Post) -> SearchDocument {
    SearchDocument::new(post.id.to_string())
        .field("body", plain_text(&post.body))
        .field("category", post.category.clone())
}

pub(super) fn index_upsert(state: &AppState, post: &Post) {
    let _ = state.index.upsert(post_search_document(post));
}

pub(super) fn index_delete(state: &AppState, id: u64) {
    let _ = state.index.delete(&id.to_string());
}

pub(super) fn rebuild_search_index(store: &FeedStore, index: &MemorySearchAdapter) {
    let _ = index.clear();
    for post in store.posts() {
        let _ = index.upsert(post_search_document(&post));
    }
}

fn query_param(query: Option<&str>, name: &str) -> Option<String> {
    let query = query?;
    for part in query.split('&') {
        let (key, value) = part.split_once('=').unwrap_or((part, ""));
        if key == name {
            return Some(percent_decode(value));
        }
    }
    None
}

fn percent_decode(raw: &str) -> String {
    let mut out = Vec::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = from_hex(bytes[i + 1]);
                let lo = from_hex(bytes[i + 2]);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h << 4) | l);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

const fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg_attr(feature = "hotpath", hotpath::measure)]
fn snippet_for(post: &Post) -> String {
    let plain = plain_text(&post.body);
    let trimmed: String = plain.chars().take(160).collect();
    if plain.chars().count() > 160 {
        format!("{trimmed}…")
    } else if trimmed.is_empty() {
        format!("#{}", post.id)
    } else {
        trimmed
    }
}

pub(super) fn handle_search(state: &AppState, request: &Request) -> Response {
    let ui = ui_for(state, request);
    let q = query_param(request.query(), "q").unwrap_or_default();
    let mut owned: Vec<(String, String, String)> = Vec::new();
    if !q.trim().is_empty() {
        let hits = state
            .index
            .query(&SearchQuery::new(q.clone()).with_limit(50))
            .unwrap_or_default();
        for hit in hits {
            let Ok(id) = hit.id().parse::<u64>() else {
                continue;
            };
            let Some(post) = state.store.get_post(id) else {
                continue;
            };
            owned.push((
                format!("/#post-{id}"),
                snippet_for(&post),
                post.category.clone(),
            ));
        }
    }
    let hit_views: Vec<SearchHitView<'_>> = owned
        .iter()
        .map(|(href, snippet, category)| SearchHitView {
            href,
            snippet,
            category,
        })
        .collect();
    html_response(
        200,
        search_page(&ui, state.matcher.collection(), &q, &hit_views),
    )
}

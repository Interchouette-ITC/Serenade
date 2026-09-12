use super::{
    DocumentIndex, MemorySearchAdapter, SearchDocument, SearchError, SearchHit, SearchQuery,
    version,
};

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn upsert_query_overwrite_delete_clear() {
    let index = MemorySearchAdapter::new();
    index
        .upsert(SearchDocument::new("1").field("body", "alpha beta"))
        .expect("upsert");
    index
        .upsert(SearchDocument::new("2").field("body", "gamma alpha"))
        .expect("upsert");

    let hits = index.query(&SearchQuery::new("alpha")).expect("query");
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().any(|hit| hit.id() == "1"));
    assert!(hits.iter().any(|hit| hit.id() == "2"));

    index
        .upsert(SearchDocument::new("1").field("body", "only beta"))
        .expect("overwrite");
    let hits = index.query(&SearchQuery::new("alpha")).expect("query");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id(), "2");

    assert!(index.delete("2").expect("delete"));
    assert!(!index.delete("2").expect("missing"));
    assert_eq!(
        index.query(&SearchQuery::new("alpha")).expect("query"),
        Vec::<SearchHit>::new()
    );

    index
        .upsert(SearchDocument::new("3").field("title", "keep"))
        .expect("upsert");
    index.clear().expect("clear");
    assert_eq!(
        index.query(&SearchQuery::new("keep")).expect("query"),
        Vec::<SearchHit>::new()
    );
}

#[test]
fn query_is_case_insensitive_and_requires_all_tokens() {
    let index = MemorySearchAdapter::new();
    index
        .upsert(
            SearchDocument::new("p1")
                .field("title", "Hello")
                .field("body", "Serenade Search"),
        )
        .expect("upsert");
    index
        .upsert(SearchDocument::new("p2").field("body", "Hello World"))
        .expect("upsert");

    let hits = index
        .query(&SearchQuery::new("hello serenade"))
        .expect("query");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id(), "p1");
    assert!(hits[0].score() >= 2);
}

#[test]
fn empty_query_returns_no_hits() {
    let index = MemorySearchAdapter::new();
    index
        .upsert(SearchDocument::new("1").field("body", "anything"))
        .expect("upsert");
    assert_eq!(
        index.query(&SearchQuery::new("")).expect("empty"),
        Vec::<SearchHit>::new()
    );
    assert_eq!(
        index.query(&SearchQuery::new("   \t")).expect("ws"),
        Vec::<SearchHit>::new()
    );
}

#[test]
fn limit_and_offset_paginate_ranked_hits() {
    let index = MemorySearchAdapter::new();
    index
        .upsert(SearchDocument::new("a").field("body", "rust rust"))
        .expect("upsert");
    index
        .upsert(SearchDocument::new("b").field("body", "rust"))
        .expect("upsert");
    index
        .upsert(SearchDocument::new("c").field("body", "rust rust rust"))
        .expect("upsert");

    let hits = index
        .query(&SearchQuery::new("rust").with_limit(2))
        .expect("query");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].id(), "c");
    assert_eq!(hits[1].id(), "a");
    assert!(hits[0].score() > hits[1].score());

    let page = index
        .query(&SearchQuery::new("rust").with_offset(1).with_limit(1))
        .expect("page");
    assert_eq!(page, vec![SearchHit::new("a", hits[1].score())]);
}

#[test]
fn empty_id_is_rejected() {
    let index = MemorySearchAdapter::new();
    let Err(err) = index.upsert(SearchDocument::new("").field("body", "x")) else {
        panic!("empty id upsert must fail");
    };
    assert!(matches!(err, SearchError::InvalidId { .. }));
    let Err(err) = index.delete("") else {
        panic!("empty id delete must fail");
    };
    assert!(matches!(err, SearchError::InvalidId { .. }));
}

#[test]
fn document_into_parts_preserves_fields() {
    let doc = SearchDocument::new("id").field("a", "1").field("b", "2");
    let (id, fields) = doc.into_parts();
    assert_eq!(id, "id");
    assert_eq!(fields.get("a").map(String::as_str), Some("1"));
    assert_eq!(fields.get("b").map(String::as_str), Some("2"));
}

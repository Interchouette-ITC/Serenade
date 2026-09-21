# Search / indexation

Basic document indexation lives in **`serenade-search`** ([#129](https://github.com/Interchouette-ITC/Serenade/issues/129), [#255](https://github.com/Interchouette-ITC/Serenade/issues/255)).

Serenade ships **contracts**, a zero-deps **in-memory** adapter, and (feature `http`) an **HTTP stub** toward Meilisearch/ES-class REST APIs. It does not embed Lucene, Algolia, Meilisearch, or Postgres FTS. Applications that outgrow the memory index implement their own adapters (or use the HTTP stub with their client).

## Pieces

| Piece                                | Role                                                                |
| ------------------------------------ | ------------------------------------------------------------------- |
| `SearchDocument`                     | Stable `id` + named text `fields`                                   |
| `SearchQuery` / `SearchHit`          | Free-text query, optional limit/offset, ranked hit                  |
| `DocumentIndex`                      | `upsert` / `delete` / `query` / `clear`                             |
| `MemorySearchAdapter`                | Case-insensitive whitespace-token AND match; score = occurrence sum |
| `HttpSearchAdapter` (feature `http`) | REST stub; apps own the HTTP client via `SearchHttpPoster`          |

Matching on the memory adapter concatenates field values. Empty / whitespace-only queries return no hits.

## HTTP stub (feature `http`)

Enable with `serenade-search` feature `http`. Serenade shapes a small REST surface; apps own the network client (no mandatory SaaS SDK).

| Piece                                       | Role                                      |
| ------------------------------------------- | ----------------------------------------- |
| `HttpSearchConfig`                          | Base URL + optional Bearer API key        |
| `SearchHttpPoster` / `MockSearchHttpPoster` | Sync request trait + test double          |
| `HttpSearchAdapter`                         | Implements `DocumentIndex` over HTTP JSON |

Endpoints (relative to base URL):

| Op     | Method   | Path              | Body / response                                                                    |
| ------ | -------- | ----------------- | ---------------------------------------------------------------------------------- |
| upsert | `PUT`    | `/documents/{id}` | `{ "id", "fields" }`                                                               |
| delete | `DELETE` | `/documents/{id}` | optional `{ "deleted": bool }` (default true on 2xx)                               |
| query  | `POST`   | `/search`         | request `{ "q", "limit?", "offset" }`; response `{ "hits": [{ "id", "score?" }] }` |
| clear  | `DELETE` | `/documents`      | empty                                                                              |

```rust
use serenade_search::{
    DocumentIndex, HttpSearchAdapter, HttpSearchConfig, MockSearchHttpPoster, SearchDocument,
    SearchQuery,
};

let poster = MockSearchHttpPoster::ok_empty();
let index = HttpSearchAdapter::new(
    HttpSearchConfig::new("https://search.example").api_key("tok"),
    poster,
);
index
    .upsert(SearchDocument::new("1").field("body", "hello"))
    .expect("upsert");
```

## Ownership

| Concern                           | Owner                                         |
| --------------------------------- | --------------------------------------------- |
| Traits + memory adapter           | Serenade (`serenade-search`)                  |
| HTTP stub shapes (feature `http`) | Serenade                                      |
| When to index domain entities     | Application                                   |
| SaaS / engine clients             | Application (or plug into `SearchHttpPoster`) |
| FrameworkBundle SaaS wiring       | **None**                                      |

## MyFeed dogfood

[`examples/MyFeed`](../examples/MyFeed) keeps a `MemorySearchAdapter` beside SQLite:

- rebuild on boot from stored posts
- upsert on create / admin edit
- delete on admin delete
- `GET /search?q=` returns escaped HTML results

Apps that outgrow the memory index can plug `HttpSearchAdapter` (feature `http`)
or their own `DocumentIndex`; indexing policy stays application-owned.

## Example

```rust
use serenade_search::{DocumentIndex, MemorySearchAdapter, SearchDocument, SearchQuery};

let index = MemorySearchAdapter::new();
index
    .upsert(SearchDocument::new("1").field("body", "hello serenade"))
    .expect("upsert");
let hits = index.query(&SearchQuery::new("serenade")).expect("query");
assert_eq!(hits[0].id(), "1");
```

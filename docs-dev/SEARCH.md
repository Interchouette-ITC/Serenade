# Search / indexation

Basic document indexation lives in **`serenade-search`** ([#129](https://github.com/Interchouette-ITC/Serenade/issues/129)).

Serenade ships **contracts** and a zero-deps **in-memory** adapter only. It does not embed Lucene, Algolia, Meilisearch, or Postgres FTS. Applications that outgrow the memory index implement their own adapters.

## Pieces

| Piece | Role |
| --- | --- |
| `SearchDocument` | Stable `id` + named text `fields` |
| `SearchQuery` / `SearchHit` | Free-text query, optional limit/offset, ranked hit |
| `DocumentIndex` | `upsert` / `delete` / `query` / `clear` |
| `MemorySearchAdapter` | Case-insensitive whitespace-token AND match; score = occurrence sum |

Matching concatenates field values. Empty / whitespace-only queries return no hits.

## Ownership

| Concern | Owner |
| --- | --- |
| Traits + memory adapter | Serenade (`serenade-search`) |
| When to index domain entities | Application |
| SaaS / engine clients | Application adapters |
| FrameworkBundle SaaS wiring | **None** |

## MyFeed dogfood

[`examples/MyFeed`](../examples/MyFeed) keeps a `MemorySearchAdapter` beside SQLite:

- rebuild on boot from stored posts
- upsert on create / admin edit
- delete on admin delete
- `GET /search?q=` returns escaped HTML results

Later load (~10k posts) may motivate a product engine adapter; that is outside this component.

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

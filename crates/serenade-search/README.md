# serenade-search

Basic document search contracts and an in-memory `MemorySearchAdapter`.

No external search engines in this crate. Applications that outgrow the memory
index implement their own adapters (Postgres FTS, Meilisearch, and so on).

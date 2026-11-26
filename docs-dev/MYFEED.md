# demo-html (HTML mode demo)

> **MyFeed**. alias to `demo-html` to align the folder with the HTML front mode.
> Package name is `serenade-demo-html`; CLI binary is also `serenade-demo-html`.
> See [FRONT-MODES.md](FRONT-MODES.md) for the full mode tree and CLI contract.

Path: [`examples/demo-html`](../examples/demo-html). Epic [#112](https://github.com/Interchouette-ITC/Serenade/issues/112).

## Goal

`cargo run -p serenade-demo-html` → open a public scrollable feed. Same _journey_ as
Symfony Demo / Fast Track: framework forms, security habits, HTML pages - not a SPA tutorial.

## Stack (locked for this demo)

| Layer     | Choice                                                                       |
| --------- | ---------------------------------------------------------------------------- |
| UI        | Server-rendered HTML + **Bootstrap 5** + **Quill** + light Clitorine helpers |
| HTTP      | Actix via `serenade-http-actix::listen`                                      |
| Forms     | `serenade-form` (CSRF default-on, `escape_html`)                             |
| View      | `serenade-view` (`path` / `asset` / `partial` for HTML builders)             |
| Search    | `serenade-search` (`MemorySearchAdapter`; `/search?q=`)                      |
| i18n      | `serenade-translation` catalogues `en` + `fr`; `/locale/{code}` cookie       |
| Profiler  | `serenade-profiler` toolbar + `/_profiler` (disable with `MYFEED_PROFILER=0`) |
| Rich text | Quill HTML sanitized with ammonia before store/render                        |
| Admin     | Cookie login on `/admin`; categories via `serenade-admin` CRUD               |
| Data      | SQLite file (default `.myfeed.sqlite`) so restarts keep posts                |

## Surfaces

- `/` feed + expandable composer + likes + comments
- `GET /?before={id}&ceiling={id}` older feed batch; `after={id}` loads toward newer posts
- `GET /?around={id}` jump to an old post without loading the archive
- `GET /feed/latest` lightweight latest-post id for hourly update checks
- `GET /posts/{id}` direct server-rendered post page
- `GET /posts/{id}/image` separately served SQLite upload, with cache revalidation
- `GET /posts/{id}/comments?after={id}` next approved-comment window; `before={id}` goes back
- `GET /search?q=` in-memory post search (body + category)
- `GET /locale/{code}` sticky UI locale cookie (`en` / `fr`); post bodies stay author language
- `POST /posts` create post (optional allowlisted media)
- `POST /posts/{id}/like` public like
- `POST /posts/{id}/comments` pending comment
- `/admin` login + pending queue; categories use `serenade-admin` list panel + CRUD under `/admin/categories`
- `/_profiler` request list and `/_profiler/{token}` detail (toolbar on HTML pages)

## Run

```bash
make html
# or: cargo run -p serenade-demo-html
# or: make myfeed  (legacy alias)
# http://127.0.0.1:8080/
```

## Continuous scrolling and resource limits

Clitorine progressively enhances the server-rendered feed. The first page and
each subsequent batch contain at most **20 posts**. Scrolling near the bottom
loads older posts automatically; scrolling back re-fetches newer windows.
Accessible Load more/Load newer links also work without JavaScript.

The browser keeps at most **60 post cards** mounted. Far-away cards and their
media are removed; bounded window metadata and spacer heights preserve the
reading position without retaining the whole archive. Focused cards, unsent
comments, active requests, and playing local videos block eviction rather than
lose work or exceed the limit. A focused embedded player is pinned until its
unload control is used. External players and browser-managed caches do not have
an application-enforceable exact memory cap.

Approved comments display **10 at a time**, with next/previous controls. Each
comment request replaces that bounded window instead of indefinitely appending
comments. New comments still require owner approval.

Feed reads use primary-key cursors, not OFFSET, and indexed per-post comment
queries. A reading-session ceiling excludes new insertions from older batches.
Images are omitted from feed/search SQL projections and HTML: existing base64
uploads remain in SQLite and are delivered through individual image URLs.
This does not migrate uploads or change their disk encoding. The text search
index still grows with the searchable archive.

The visible tab checks for newer posts **once per hour**, never while hidden.
A New posts button lets the reader return to the latest feed; it does not
automatically insert posts or move the current reading position. Browser Back
restores the visible post and offset after Search or Edit. Search uses direct
post pages, and Clitorine resolves legacy `/#post-{id}` links with a bounded
around-post request. Without JavaScript, use a direct post URL or the post-jump
form; URL fragments are not sent to the server.

For hosting, set a persistent `MYFEED_DB` path and back up that SQLite database;
it contains uploads as well as posts and comments. Use your own
`MYFEED_CSRF_SECRET` and `MYFEED_ADMIN_TOKEN`, and disable the development
profiler with `MYFEED_PROFILER=0`. No separate upload folder or external service
is required.

Embeds: YouTube (nocookie embed), SoundCloud, direct image / `.mp4` URLs. See [FORMS.md](FORMS.md), [VIEW.md](VIEW.md), [SECURITY.md](SECURITY.md), [I18N.md](I18N.md), [SEARCH.md](SEARCH.md), and [ADMIN.md](ADMIN.md).

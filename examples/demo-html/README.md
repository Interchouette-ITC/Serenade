# demo-html (MyFeed)

Beginner Serenade demo: an **open public feed** you can self-host.

Server-rendered HTML + Bootstrap 5 + Quill + [`serenade-form`](../../crates/serenade-form) (CSRF + HTML escape) + Actix `listen`.

## Run

```bash
# preferred mode command:
make html
# or: make myfeed (legacy alias)
# direct package run:
cargo run -p serenade-demo-html
```

Open <http://127.0.0.1:8080/>.

The mode command defaults to `demo-html`. Named app arguments are for choosing among multiple
registered HTML apps; today there is only one. Use `make html-all` to run every registered HTML
app.

The demo stores posts in `.myfeed.sqlite` and only inserts the starter post when the database is
empty. Code changes rebuild normally, but changing the seed text will not update an existing
database. To preview the starter post in a fresh database without replacing your existing posts:

```bash
MYFEED_DB=/tmp/demo-html-preview.sqlite make html
```

### Optional performance profiling

Default builds (and CI) do **not** enable hotpath. For a local function-timing report:

```bash
cargo run -p serenade-demo-html --features hotpath
```

See [`docs-dev/PERF.md`](../../docs-dev/PERF.md) and [hotpath.rs](https://hotpath.rs/).

| Env                  | Default            | Role                                 |
| -------------------- | ------------------ | ------------------------------------ |
| `MYFEED_BIND`        | `127.0.0.1:8080`   | Listen address                       |
| `MYFEED_CSRF_SECRET` | (dev secret)       | HMAC CSRF key                        |
| `MYFEED_ADMIN_TOKEN` | `myfeed-dev-admin` | Admin sign-in token                  |
| `MYFEED_DB`          | `.myfeed.sqlite`   | SQLite file (posts survive restarts) |

## What you get

- Wider public wall with purple Bootstrap theme
- Composer opens from the top input (Bootstrap collapse)
- Quill WYSIWYG (bold / lists / links) + emoji toolbar icon
- Optional image upload (up to 1 MiB) / YouTube / SoundCloud / `.mp4` / image URL
- Public likes; comments pending until approved
- `/admin` cookie login, moderation queue
- Category CRUD via `serenade-admin` (`/admin/categories`)
- Admin Edit / Delete on each post (after sign-in); posts show date
- `GET /search?q=` in-memory search over post body + category

## Admin

Open `/admin`, enter the token (default `myfeed-dev-admin`). Approve comments, manage categories under `/admin/categories` (list / new / edit / delete with CSRF), edit or delete posts. Bearer still works for scripts.

## Stack

| Layer              | Choice                                                            |
| ------------------ | ----------------------------------------------------------------- |
| HTTP               | `serenade-http-actix::listen`                                     |
| Forms / CSRF / XSS | `serenade-form` + `HmacCsrfTokenManager` + ammonia for Quill HTML |
| UI                 | Bootstrap 5 + Quill (+ Clitorine helpers under `assets/`)         |
| Persistence        | SQLite (default `.myfeed.sqlite`)                                 |

See epic [#112](https://github.com/Interchouette-ITC/Serenade/issues/112) and `docs-dev/MYFEED.md`.

## Component dogfood

| Crate | How this sample uses it |
| --- | --- |
| `serenade-string` | Category names stored as slugs |
| `serenade-notifier` | `NullTransport` SMS on comment approve |
| `serenade-filesystem` / `serenade-finder` | Translation dir check + `*.toml` discovery |
| `serenade-workflow` + `ExpressionGuard` | Comment approve pending → approved (blocks subject `guest`) |

## About / inspiration

Thanks to [JARVI3 Pulse](https://jarvi3.com/pulse) for inspiring the feed-first shape of this demo. JARVI3 Pulse is original work - not affiliated with Serenade. The in-app **About** link in the footer repeats this note.

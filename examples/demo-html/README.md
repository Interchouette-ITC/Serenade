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

Click a post image to view it at its original size in a modal. Images larger
than the viewport can be scrolled; the image link also works without JavaScript.

### Docker

From the repository root, build and run MyFeed with Compose:

```bash
make myfeed-up
# To build the image without starting the service:
make myfeed-image
```

Open <http://127.0.0.1:8080/>. The database is stored in the Compose-managed
`myfeed-data` volume and survives container restarts. Stop the service with `make myfeed-down`.
The container uses the app's documented development admin token and CSRF secret by default.
Set `MYFEED_ADMIN_TOKEN` and `MYFEED_CSRF_SECRET` in the environment before `make myfeed-up`
for any non-local deployment.

CI and local publishing use the same Make targets:
`make docker-build-dev` builds and tags `:dev` / `:latest` for Docker Hub and GHCR
(`make myfeed-image` is an alias).

The mode command defaults to `demo-html`. Named app arguments are for choosing among multiple
registered HTML apps; today there is only one. Use `make html-all` to run every registered HTML
app.

The demo stores posts in `.myfeed.sqlite` and inserts the "Hello World" and "Meet Serenade" starter posts when the database is
empty. Code changes rebuild normally, but changing the seed text will not update an existing
database. To preview the starter post in a fresh database without replacing your existing posts:

```bash
MYFEED_DB=/tmp/demo-html-preview.sqlite make html
```

To customize the appearance without changing the shared app, set
`MYFEED_THEME_CSS` to a readable CSS file. It is appended after the built-in
theme; changes are served on refresh without restarting the server. An invalid
path is reported at startup, and a file that becomes unreadable returns an
explicit error instead of silently using the default theme. If a `favicon.ico`
file exists alongside the configured stylesheet at startup, MyFeed serves it
instead of the default icon.

Theme assets placed alongside the stylesheet can be referenced as
`/assets/theme/filename.webp` or `/assets/theme/filename.js` (PNG, JPEG, WebP,
GIF and JavaScript are supported). These files are public; images are cached
with revalidation, while JavaScript is served without caching. Filenames use
letters, digits, hyphens or underscores before the extension. Symbolic links
are not served. Only use JavaScript from trusted sources.

If `header.webp` exists alongside the configured theme stylesheet at startup,
it is preloaded on full feed and search pages. Set `MYFEED_THEME_PRELOAD_IMAGE`
to a different theme image filename to override it. This requires
`MYFEED_THEME_CSS` and should match the image used by the theme.

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
| `MYFEED_ADMIN_PATH`  | `admin`            | Admin URL segment; hides public admin links when changed |
| `MYFEED_DB`          | `.myfeed.sqlite`   | SQLite file (posts survive restarts) |
| `MYFEED_THEME_CSS`   | (unset)            | Optional CSS file appended to MyFeed |
| `MYFEED_THEME_PRELOAD_IMAGE` | `header.webp` if present | Override the theme image preloaded on public headers |
| `MYFEED_FOOTER_FILE` | (unset)            | Optional trusted HTML file for the feed footer |

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
When `MYFEED_ADMIN_PATH` is set to a different segment, use that segment instead
of `admin` for the whole admin area. The URL is not authentication; set a strong
admin token.

## Stack

| Layer              | Choice                                                            |
| ------------------ | ----------------------------------------------------------------- |
| HTTP               | `serenade-http-actix::listen`                                     |
| Forms / CSRF / XSS | `serenade-form` + `HmacCsrfTokenManager` + ammonia for Quill HTML |
| UI                 | Bootstrap 5 + Quill (+ Clitorine helpers under `assets/`)         |
| Persistence        | SQLite (default `.myfeed.sqlite`)                                 |

See epic [#112](https://github.com/Interchouette-ITC/Serenade/issues/112) and `docs-dev/MYFEED.md`.

## Component dogfood

| Crate                                     | How this sample uses it                                     |
| ----------------------------------------- | ----------------------------------------------------------- |
| `serenade-string`                         | Category names stored as slugs                              |
| `serenade-notifier`                       | `NullTransport` SMS on comment approve                      |
| `serenade-filesystem` / `serenade-finder` | Translation dir check + `*.toml` discovery                  |
| `serenade-workflow` + `ExpressionGuard`   | Comment approve pending → approved (blocks subject `guest`) |

## About / inspiration

Thanks to [JARVI3 Pulse](https://jarvi3.com/pulse) for inspiring the feed-first shape of this demo. JARVI3 Pulse is original work - not affiliated with Serenade. The in-app **About** link in the footer repeats this note.

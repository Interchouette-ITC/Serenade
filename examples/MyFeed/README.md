# MyFeed

Beginner Serenade demo: an **open public feed** you can self-host.

Server-rendered HTML + Bootstrap 5 + Quill + [`serenade-form`](../../crates/serenade-form) (CSRF + HTML escape) + Actix `listen`.

## Run

```bash
cargo run -p my_feed
# or: make myfeed
```

Open <http://127.0.0.1:8090/>.

| Env                  | Default            | Role                                 |
| -------------------- | ------------------ | ------------------------------------ |
| `MYFEED_BIND`        | `127.0.0.1:8090`   | Listen address                       |
| `MYFEED_CSRF_SECRET` | (dev secret)       | HMAC CSRF key                        |
| `MYFEED_ADMIN_TOKEN` | `myfeed-dev-admin` | Admin sign-in token                  |
| `MYFEED_DB`          | `.myfeed.sqlite`   | SQLite file (posts survive restarts) |

## What you get

- Wider public wall with purple Bootstrap theme
- Composer opens from the top input (Bootstrap collapse)
- Quill WYSIWYG (bold / lists / links) + emoji toolbar icon
- Optional image upload / YouTube / SoundCloud / `.mp4` / image URL
- Public likes; comments pending until approved
- `/admin` cookie login, moderation queue, category manager
- Admin Edit / Delete on each post (after sign-in); posts show date
- `GET /search?q=` in-memory search over post body + category

## Admin

Open `/admin`, enter the token (default `myfeed-dev-admin`). Manage categories, approve comments, edit or delete posts. Bearer still works for scripts.

## Stack

| Layer              | Choice                                                            |
| ------------------ | ----------------------------------------------------------------- |
| HTTP               | `serenade-http-actix::listen`                                     |
| Forms / CSRF / XSS | `serenade-form` + `HmacCsrfTokenManager` + ammonia for Quill HTML |
| UI                 | Bootstrap 5 + Quill (+ Clitorine helpers under `assets/`)         |
| Persistence        | SQLite (default `.myfeed.sqlite`)                                 |

See epic [#112](https://github.com/Interchouette-ITC/Serenade/issues/112) and `docs-dev/MYFEED.md`.

## About / inspiration

Thanks to [JARVI3 Pulse](https://jarvi3.com/pulse) for inspiring the feed-first shape of this demo. JARVI3 Pulse is original work - not affiliated with Serenade. The in-app **About** link in the footer repeats this note.

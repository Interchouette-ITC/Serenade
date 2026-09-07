# MyFeed beginner demo

Path: [`examples/MyFeed`](../examples/MyFeed). Epic [#112](https://github.com/Interchouette-ITC/Serenade/issues/112).

## Goal

`cargo run -p my_feed` → open a public scrollable feed. Same _journey_ as Symfony Demo / Fast Track: framework forms, security habits, HTML pages - not a SPA tutorial.

## Stack (locked for this demo)

| Layer     | Choice                                                                       |
| --------- | ---------------------------------------------------------------------------- |
| UI        | Server-rendered HTML + **Bootstrap 5** + **Quill** + light Clitorine helpers |
| HTTP      | Actix via `serenade-http-actix::listen`                                      |
| Forms     | `serenade-form` (CSRF default-on, `escape_html`)                             |
| Rich text | Quill HTML sanitized with ammonia before store/render                        |
| Admin     | Cookie login on `/admin` (Bearer still accepted); category manager           |
| Data      | SQLite file (default `.myfeed.sqlite`) so restarts keep posts                |

## Surfaces

- `/` feed + expandable composer + likes + comments
- `POST /posts` create post (optional allowlisted media)
- `POST /posts/{id}/like` public like
- `POST /posts/{id}/comments` pending comment
- `/admin` login + pending queue + categories; approve / reject

## Run

```bash
make myfeed
# http://127.0.0.1:8090/
```

Embeds: YouTube (nocookie embed), SoundCloud, direct image / `.mp4` URLs. See [FORMS.md](FORMS.md) and [SECURITY.md](SECURITY.md).

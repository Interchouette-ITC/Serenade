# View helpers

Optional HTML view helpers live in **`serenade-view`**. Escape defaults stay in **`serenade-form`** ([FORMS.md](FORMS.md)); this crate re-exports them for a single import surface.

There is **no** mandatory template engine. Apps compose HTML with plain builders, Askama, Tera, minijinja, or similar.

## Concept map

| Twig / Symfony | Serenade |
| --- | --- |
| `path('route', {id: 1})` | `path(&routes, "route", &[("id", "1")])` |
| `asset('css/app.css')` | `asset("css/app.css")` / `AssetConfig` |
| `{% include %}` | `partial(\|\| { … })` |
| autoescape | `escape_html` / `escape_attr` (from Forms; re-exported) |

## Reverse routing

`RouteCollection::generate` (in `serenade-http`) substitutes `{param}` segments and percent-encodes values. `path()` is the view-layer wrapper.

```rust
use serenade_view::path;

let href = path(&routes, "admin_post_edit_get", &[("id", "42")])?;
```

Missing routes return HTTP-style **404**; missing or unused parameters return **400**.

## Assets

Default base is `/assets`. Rejects `..` and scheme-like paths (`:`). Optional `?query` is kept for cache-busting.

```rust
use serenade_view::{asset, AssetConfig};

assert_eq!(asset("myfeed.css")?, "/assets/myfeed.css");
let url = AssetConfig::with_base("/static").join("js/app.js")?;
```

## Partials

`partial` runs a closure and returns the fragment. Use it to nest named HTML pieces without an engine:

```rust
use serenade_view::{escape_html, partial};

let chip = partial(|| format!("<span>{}</span>", escape_html(label)));
```

## Not these

| Concern | Where |
| --- | --- |
| Console / `debug:*` | [CONSOLE.md](CONSOLE.md) |
| Web Debug Toolbar / profiler | [PROFILER.md](PROFILER.md) |
| CSRF + form bind | [FORMS.md](FORMS.md) |

## Issue

[#111](https://github.com/Interchouette-ITC/Serenade/issues/111).

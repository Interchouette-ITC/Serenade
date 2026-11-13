# View helpers and complete documents

Optional HTML view helpers live in **`serenade-view`**. Escape defaults stay in
**`serenade-form`** ([FORMS.md](FORMS.md)); this crate re-exports them for a single import
surface. Serenade does not require a template engine. Applications may use Rangular AOT,
Leptos views, or another engine through the document-template boundary.

## Concept map

| Twig / Symfony | Serenade |
| --- | --- |
| `path('route', {id: 1})` | `path(&routes, "route", &[("id", "1")])` |
| `asset('css/app.css')` | `asset("css/app.css")` / `AssetConfig` |
| `{% include %}` | `partial(|| { … })` |
| autoescape | `escape_html` / `escape_attr` (from Forms; re-exported) |

## Reverse routing

`RouteCollection::generate` (in `serenade-http`) substitutes `{param}` segments and
percent-encodes values. `path()` is the view-layer wrapper.

```rust
use serenade_view::path;

let href = path(&routes, "admin_post_edit_get", &[("id", "42")])?;
```

Missing routes return HTTP-style **404**; missing or unused parameters return **400**.

## Assets

Default base is `/assets`. Rejects `..` and scheme-like paths (`:`). Optional `?query` is
kept for cache-busting.

```rust
use serenade_view::{asset, AssetConfig};

assert_eq!(asset("myfeed.css")?, "/assets/myfeed.css");
let url = AssetConfig::with_base("/static").join("js/app.js")?;
```

## Partials

`partial` runs a closure and returns the fragment. Use it to nest named HTML pieces:

```rust
use serenade_view::{escape_html, partial};

let chip = partial(|| format!("<span>{}</span>", escape_html(label)));
```

## Complete documents

`Document` composes application-owned head and body output. With `leptos-ssr`, it renders
both under one request-scoped Leptos owner, injects metadata, and returns a
`RenderedDocument`. Rust controllers still own route selection and page data; they do not
assemble the final HTML string or static output path.

| Concern | Owner |
| --- | --- |
| Route enumeration and page data | Application |
| Head/body templates and custom document structure | Application |
| Head metadata context and composition | `serenade-view` |
| Mapping an SSG URL route to `index.html` output | `StaticDocumentWriter` |
| Exact Trunk asset URLs and build metadata | `TrunkAssetManifest` |

### Optional features

| `serenade-view` feature | Provides |
| --- | --- |
| No feature | Engine-neutral `Document`, `DocumentTemplate`, `RenderedDocument`, static output writer, manifest reader, and existing view helpers. No Leptos dependency is enabled. |
| `leptos-ssr` | `initialize_leptos_ssr`, `Document::render_leptos`, and `Document::render_with_leptos`. |
| `leptos-client` | Client head components such as `ManagedHead`, `ManagedTitle`, `ManagedMeta`, `ManagedLink`, and `JsonLd`. |

The application selects its Leptos CSR or hydration features separately. Rangular AOT is
also opt-in; its head compiler is a build-time API, not a feature of `serenade-view`.

### Generic head extension point

`DocumentTemplate` is the engine-neutral composition boundary: the application or template
adapter supplies its head and body outputs, and the document template owns their structure.
For Leptos client rendering and hydration, the optional `leptos-client` feature provides
`ManagedHead` as a generic head lifecycle extension point. The application supplies the active
head view as its children; `ManagedTitle`, `ManagedMeta`, and `ManagedLink` register reactive
entries. Entries with the same stable `head_key` replace earlier entries, while unkeyed
repeatable entries remain and registered entries are removed when their view unmounts.

This lifecycle is not tied to Rangular: any Leptos view or template adapter can supply the head
view. Rangular AOT is one producer of those views. On the server,
`Document::render_with_leptos()` wraps the supplied head view in the same managed context so
the initial document and browser updates share the head contract.
Server-rendered `ManagedMeta`, `ManagedLink`, and `JsonLd` entries carry
`data-serenade-head="ssr"`. Applications that remount their head during body hydration
must remove only those owned entries, never all matching metadata in the document.
Raw application-owned head elements need an explicit ownership marker if they are to
be replaced. Leptos claims the existing title node through its title lifecycle.

### Minimal Rangular head and body

Keep the templates in the app and compile them during the build. Use the head-specific
compiler for the document head; compile the page body with the ordinary Rangular AOT
compiler.

`src/page-head.html`:

```html
<title>{{ title }}</title>
<meta name="description" content="{{ description }}" />
<link rel="canonical" href="{{ canonical }}" />
<script type="application/ld+json">{{ json_ld }}</script>
```

In `build.rs`, compile the head template and write `result.code` to `OUT_DIR`, failing the
build when `result.ok()` is false:

```rust
let result = rangular_aot::compile_head_named(
    include_str!("src/page-head.html"),
    "src/page-head.html",
    "page_head_view",
);
assert!(result.ok(), "{:?}", result.issues);
```

The normal body template can use the same host-backed data:

```html
<main>
  <h1>{{ title }}</h1>
  <p>{{ summary }}</p>
</main>
```

The generated functions take a Rangular `HostCell`; the app implements `Host` to expose
the current page data. Initialize the SSR executor once during process startup, not once
per request. Then build one document per request:

```rust
use rangular_aot::HostCell;
use serenade_view::{Document, initialize_leptos_ssr};

initialize_leptos_ssr()?;

let host = HostCell::new(PageHost::from(page_data));
let rendered = Document::new(
    move || page_head_view(host.clone()),
    move || page_body_view(host),
)
.render_leptos()?;
let response = rendered.into_response();
```

`PageHost`, `page_head_view`, and `page_body_view` are application/generated names.
Rangular compiles these templates to Leptos views; no runtime template interpreter or
second HTML-string concatenation step is involved.

### Client-managed route head

For CSR or hydration, place `ManagedHead` in the document `<head>`. Use explicit stable
keys for singleton entries that a route may replace. Unkeyed entries remain repeatable,
which is useful for multiple Open Graph images.

```rust
view! {
    <ManagedHead>
        <ManagedTitle text=title />
        <ManagedMeta
            head_key="meta:name:description"
            name="description"
            content=description
        />
        <ManagedLink
            head_key="link:canonical"
            rel="canonical"
            href=canonical
        />
        <ManagedMeta property="og:image" content=first_image />
        <ManagedMeta property="og:image" content=second_image />
    </ManagedHead>
}
```

When a mounted route registers an entry with the same key, it takes precedence; removing
that route restores the still-mounted shared entry. Entries without keys are not
coalesced. The Rangular AOT head compiler emits ordinary head elements; use the managed
components when client-side singleton replacement by stable key is required.

### Custom document structure

`render_leptos()` supplies a default `<!DOCTYPE html><html lang="en">` document.
`render_with_leptos()` is for an application-owned document shell; its `head` and `body`
slots are rendered in the same request-scoped owner and the returned view must be the
complete document.

```rust
let rendered = Document::new(
    move || page_head_view(host.clone()),
    move || page_body_view(host),
)
.render_with_leptos(|head, body| {
    view! {
        <!DOCTYPE html>
        <html lang="fr">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                {head()}
            </head>
            <body>
                <div id="storefront">{body()}</div>
            </body>
        </html>
    }
})?;
```

This is also the boundary for an AOT-generated Rangular document component with head and
body projection slots. For a different template engine, implement `DocumentTemplate` and
pass the rendered head/body outputs to that engine. The application owns the document
structure and must use that engine's escaping rules; Serenade does not sanitize an
arbitrary HTML string.

## Delivery behavior

`HTML`, `SPA`, `SSR`, `SSG`, and `Universal` describe related but distinct concerns.
`SSR` is a rendering strategy, not another Serenade mode command.

| Delivery | What is delivered | Document/head behavior |
| --- | --- | --- |
| HTML | A server-hosted app handles requests. `serenade-html` launches the app; it does not force Leptos or SSR. | An app may render a complete document per request and return `RenderedDocument::into_response()`. |
| SPA | Trunk's static shell and client bundle; page content and route changes run in the browser. | Client head helpers update browser metadata. Route-specific metadata is not in the initial shell unless it is fixed there or prerendered. |
| SSR | A server renders the current route for each request. This is a strategy usable by a server app, not a separate CLI mode. | The initial response contains the complete route-specific document; hydration is optional. |
| SSG | Trunk builds the frontend bundle, then an app prerenders its declared routes to files. | The app owns route enumeration/data; `StaticDocumentWriter` owns route-to-file mapping. |
| Universal | A Trunk frontend and backend server are run together. | The backend can render route-specific documents per request and serve the frontend's exact assets for hydration. |

Use SSR or SSG when crawlers must receive route-specific metadata in the initial HTML.
A client-only SPA can update metadata after it starts, but that is not equivalent to
server-rendered or prerendered route metadata.

### HTTP responses, SSG output, and generated assets

`RenderedDocument::into_response()` returns status 200 with
`text/html; charset=utf-8`; use `into_response_with_status(status)` when the route needs
another status. `as_html()` reads the complete document, and `into_static_output()` returns
it for other output APIs.

The `serenade-ssg` command supplies `SERENADE_STATIC_OUTPUT_DIR` to the prerender process.
Construct the writer from that environment and pass the URL route, not a filesystem path:

```rust
let writer = StaticDocumentWriter::from_env()?;
let rendered = render_route(&page)?;
writer.write("/about", rendered)?;
```

`/` maps to `index.html`; `/about` and `/about/` map to `about/index.html`. Query strings,
fragments, traversal, and unsafe path segments are rejected. The writer creates parent
directories and returns errors for invalid routes or failed filesystem writes. It never
writes outside the output directory through a symbolic link: a route directory that is a
symbolic link is rejected, and each document is written to a new temporary file that is
then renamed over `index.html`, replacing a symbolic link instead of following it. The
configured output directory itself may be a symbolic link. Route
selection and page data remain the application's responsibility.

`serenade-ssg` and `serenade-universal` provide
`SERENADE_TRUNK_ASSET_MANIFEST`; Universal also provides `SERENADE_TRUNK_DIST_DIR`.
Read `TrunkAssetManifest::read_from_env()` once at startup to get the exact public URLs,
preloads, and bootstrap metadata emitted by Trunk. The manifest may describe an HTML-only
build with no WASM bundles. `read_from_env()` returns an error when the environment,
manifest, or schema is missing or invalid; `validate_output(dist)` additionally checks
the generated files, digests, and URLs against the output directory, and rejects any
referenced file whose real path (after following symbolic links) is outside it. Do not reconstruct
hashed filenames or silently fall back to guessed asset paths.
The manifest public URL must be a root-relative path (`/`, `/app/`) or an absolute
`http`/`https` URL; other schemes (`javascript:`, `data:`), protocol-relative `//host`
prefixes, whitespace, quotes, backslashes, and angle brackets are rejected when the
manifest is parsed. Every asset, WASM, initializer, and preload URL must sit under the
manifest public URL at a path boundary: with a `/app` public URL, `/app/app.js` is
accepted but `/apple.js` is rejected. Select the
Universal bootstrap module by an explicit marker value, such as
`data-serenade-universal-bootstrap="true"`, not by the attribute's mere presence.
A server that hosts the output itself must resolve request paths with
`resolve_output_file(dist, path)`, rather than joining the raw request path onto `dist`.
It applies the same public-URL rules as `validate_output` and returns the real file path
only when that file, after following symbolic links, is inside `dist`, including files
the manifest does not list. `output_path_for_url(path)` returns the relative mapping alone. With a
`/app/` public URL, `/app/app.js` is the output file `app.js`, and the public root
(`/app/` or `/app`) is the HTML entry point named by the manifest.
Application templates must build frontend file URLs (icons, images, stylesheets, links
to the SPA root) from the same manifest `public_url` rather than hard-coding `/`; when the
page hydrates, hand that value to the client (for example as an attribute on the bootstrap
script) so both sides render identical URLs.

### Migration from marker-based rendering

Keep the application parts that are already correct: route enumeration, page data, and
the Rangular head/body templates. Replace the consumer code that locates
`head_marker`/`content_marker`, calls `split_once`, or substitutes strings:

1. Compile the head template with `compile_head_named` and the body with the normal
   Rangular AOT compiler. Keep both in the app's build inputs.
2. Create the page-data `HostCell` and pass the generated head/body views to
   `Document::new(...).render_leptos()`; use `render_with_leptos()` when the document
   shell is custom.
3. For request-time rendering, return `RenderedDocument::into_response()` from the
   handler. Do not split or patch a Trunk HTML file per request.
4. For SSG, keep the app's route loop but replace manual output paths with
   `StaticDocumentWriter::from_env()?.write(route, rendered_document)`.
5. In Universal, read the supplied Trunk manifest rather than guessing hashed paths.
   For SPA-only delivery, keep Trunk in charge of the initial shell and use the client
   head lifecycle for browser route changes.

This guide documents adoption only. Existing websites and demos remain separate
migration work; no application migration is implied.

## Trusted content and errors

Treat route data as untrusted. Let Rangular/Leptos render interpolations as text or
attributes; do not inject untrusted values as markup, raw script bodies, or manually
assembled HTML. When manually producing a small HTML fragment, use `escape_html` for
text and `escape_attr` for attribute values. A custom `DocumentTemplate` or another
template engine must apply its own escaping; `RenderedDocument` is a delivery wrapper,
not a sanitizer.

JSON-LD must be a complete, valid serialized JSON value, not interpolated fragments.
Rangular's head compiler supports a single interpolation for the complete JSON-LD body
and escapes script-breaking characters. The `JsonLd` component parses its input at render
time and panics if it is invalid JSON, so serialize and validate the value before
constructing the view. Do not place untrusted input in other script bodies; Rangular
preserves non-JSON-LD script text as raw content.

Propagate rendering, initialization, asset-manifest, and static-write errors. SSR
initialization can fail if an executor is already installed; applications using a
different runtime should initialize its Leptos executor instead. Missing or malformed
Trunk metadata and failed route writes are errors, not successful empty-document
fallbacks. Rangular AOT diagnostics should fail the app build.

## Not these

| Concern | Where |
| --- | --- |
| Console / `debug:*` | [CONSOLE.md](CONSOLE.md) |
| Web Debug Toolbar / profiler | [PROFILER.md](PROFILER.md) |
| CSRF + form bind | [FORMS.md](FORMS.md) |
| Mode layout and hosting | [FRONT-MODES.md](FRONT-MODES.md) |

## Issue

[#111](https://github.com/Interchouette-ITC/Serenade/issues/111).

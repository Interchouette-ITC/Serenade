# Front modes

Serenade examples are structured as apps and libs under `examples/`, following an Nx-inspired
workspace layout. Each deployable app has a **mode** (`html` | `spa` | `ssg` | `universal`)
that maps to a dedicated CLI command.

## Examples tree (locked)

```text
examples/
  demo-app/                 # bundle + DI + console + RouteLoader skeleton (NOT a front mode)
  demo-html/                # mode HTML (server-rendered HTML + CSRF + forms; MyFeed)
  demo-monorepo/            # modes SPA + SSG sharing UI lib
    apps/
      demo-spa/             # Trunk CSR → dist/index.html + wasm + js glue
      demo-ssg/             # SSG → per-route HTML + hydrate wasm
    libs/
      demo-ui/              # shared panels/chrome used by demo-spa and demo-ssg
  demo-universal/           # mode Universal (needs a server process)
    apps/
      demo-front/           # Leptos CSR at `/`; hydrates the SSR document at `/cms/hello`
      demo-back/            # API/assets server; request-time SSR at `/cms/hello`
    libs/
      demo-contract/        # shared serde DTO(s) for GET /hello (and similar)
```

### What each folder demos

| Path | Demoes | Does not demo |
| --- | --- | --- |
| `demo-app` | Serenade **bundle**: config packages, Extension, DI service, routes, console commands, event subscriber | HTML site, wasm SPA, SSG, product shop |
| `demo-html` | Mode **HTML**: server-rendered pages, forms/CSRF, Clitorine/Bootstrap (MyFeed) | Leptos, rangular, Trunk dist |
| `demo-monorepo` | Multi-app front workspace: **apps** deployable, **libs** shared; SPA and SSG share UI | Backend API, Universal listen |
| `demo-monorepo/apps/demo-spa` | Mode **SPA**: hello + button, client-rendered wasm | Per-route HTML files on disk |
| `demo-monorepo/apps/demo-ssg` | Mode **SSG**: same UI via `demo-ui`, prerendered HTML per route + hydrate | Running Serenade HTTP server |
| `demo-monorepo/libs/demo-ui` | Shared Leptos UI for spa and ssg | Standalone deploy |
| `demo-universal` | Mode **Universal**: client-rendered `/` plus request-time SSR and hydration at `/cms/hello` | Full rustashop; flat CDN-only marketing |
| `demo-universal/apps/demo-front` | Leptos client-rendered `/`; WASM bundle also hydrates the server-rendered `/cms/hello` document | SSG export |
| `demo-universal/apps/demo-back` | Boot Serenade kernel, serve `GET /hello` (JSON) and frontend assets, render `/cms/hello` per request | HTML demo (demo-html), console tutorial |
| `demo-universal/libs/demo-contract` | Shared request/response types front↔back | UI components |

### Why demo-html is not inside demo-monorepo

`demo-html` is the **HTML mode** dogfood (server HTML + Clitorine). It is not a Leptos app and
does not share `demo-ui`. Renamed from MyFeed to `demo-html` to align the folder with the mode
name. Package id is now `serenade-demo-html`; folder name in docs and CLI is **demo-html**.

### Why Universal is not inside demo-monorepo

Universal **needs a backend process**. The SPA/SSG monorepo is static/wasm-oriented. Universal
lives in its own tree with `demo-front` + `demo-back` + `demo-contract`.

## Mode → CLI → folder

| Mode | CLI | Default app | Build artefact |
| --- | --- | --- | --- |
| HTML | `serenade-html` | `examples/demo-html` | Running server (port bind) |
| SPA | `serenade-spa` | `examples/demo-monorepo/apps/demo-spa` | `dist/` shell + wasm + js |
| SSG | `serenade-ssg` | `examples/demo-monorepo/apps/demo-ssg` | `dist/` HTML per route + wasm hydrate |
| Universal | `serenade-universal` | `examples/demo-universal` | Running server serving API + front |

SSR is a rendering strategy: the server renders a route for each request. SSG renders declared
routes during the build and writes static HTML files. AOT compiles templates into view code; it
does not render route HTML or choose between SSR and SSG. The SSG demo uses Leptos's server
renderer at build time for its route files, then hydrates the shared `demo-ui` in the browser.
Its prerenderer builds each route's head and WASM loader from the Trunk asset manifest and
fails the build when the manifest cannot hydrate, for example when Trunk script injection
(`inject_scripts = false` in `Trunk.toml`) is disabled.
Rangular AOT templates also compile to Leptos views, which Leptos can render and hydrate; see
the rangular SPEC. HTML, SPA, SSG, and Universal describe delivery shapes, not interchangeable
rendering strategies. See [VIEW.md](VIEW.md) for the complete-document API, head extension
point, asset handoff, and marker-to-framework migration guide.

## Mode commands

All four mode commands use the same selection shape: no app name selects that mode's default
app, an app name selects it explicitly, and `--all` selects every app registered for that mode.
The name argument is useful when a mode has multiple registered apps; currently each mode has
one registered demo app, so naming it is equivalent to using the default command.

| Mode | Default | Named app | All apps | Make |
| --- | --- | --- | --- | --- |
| HTML | `serenade-html` | `serenade-html demo-html` | `serenade-html --all` | `make html` / `make html-all` |
| SPA | `serenade-spa` | `serenade-spa demo-spa` | `serenade-spa --all` | `make spa` / `make spa-all` |
| SSG | `serenade-ssg` | `serenade-ssg demo-ssg` | `serenade-ssg --all` | `make ssg` / `make ssg-all` |
| Universal | `serenade-universal` | `serenade-universal demo-universal` | `serenade-universal --all` | `make universal` |

HTML runs the app server (default `127.0.0.1:8080`); `--all` starts all selected HTML apps.
SPA builds with Trunk to `examples/demo-monorepo/apps/demo-spa/dist/`.
SPA and Universal frontend builds minify generated `.html` files in `dist/` after Trunk.
SSG builds a hydration bundle, prerenders the selected app's declared routes, and minifies
the resulting `.html` files. The prerenderer reads the output directory from
`SERENADE_STATIC_OUTPUT_DIR` and asset tags from the validated Trunk manifest, so a non-default
`--dist` or `TRUNK_BUILD_DIST` is prerendered where Trunk wrote it. It renders each route as a
complete Leptos document with its own managed title, description, and canonical link; it does
not fill markers in Trunk's generated HTML. The `serenade-minify` component minifies inline classic and
module JavaScript as well as HTML, preserving HTML comments (including Leptos hydration
markers) and meaningful text whitespace. External scripts and non-JavaScript script data
blocks are left intact; CSS is not minified here.
Universal builds the `demo-front` Trunk bundle and starts `demo-back` on `127.0.0.1:8080`;
the backend serves `/hello` JSON, frontend assets, and `/cms/hello`. The `/` route remains
client-rendered. `/cms/hello` is rendered by the backend on each request from Rangular-authored,
AOT-compiled document, head, and body templates; Leptos renders those views to HTML, then
hydrates the existing body in the browser. The head uses Serenade's generic Leptos
`ManagedHead` lifecycle, so other Leptos view or template adapters can supply head content
without requiring Rangular. See [VIEW.md](VIEW.md#generic-head-extension-point). The existing
`examples/demo-monorepo/apps/demo-ssg` and Rust.Contact remain unchanged; neither is migrated
by Universal. The demo's HTTP kernel uses `HtmlMinifier` for runtime `text/html` responses.
Other server apps can opt in by adding the same middleware to their kernel.

The current HTML and Universal examples both default to `127.0.0.1:8080`; running both at once
requires changing one app's bind address.

## Multi-app selection and future hosting

The per-mode `--all` flag selects every registered app of that mode for that mode CLI's normal
action. It does not create one combined site or production host.

The planned workspace selector has this contract:

```text
serenade run-many -t build --apps demo-spa,demo-ssg
serenade run-many -t test --apps demo-html,demo-spa
serenade run-many -t build --mode spa
serenade run-many -t build --all-modes
```

`--apps` names a comma-separated set of registered apps. `--mode` selects all apps of one mode;
`--all-modes` selects every app. `--apps` may be combined with `--mode` as an intersection
filter. A selector is required, conflicting selectors are errors, unknown names and mode
mismatches fail explicitly, and empty selections are errors. **This `run-many` command is not
implemented yet**; use the existing per-mode commands and `--all` meanwhile.

For hosting, keep build orchestration separate from runtime routing. SPA and SSG outputs are
static artifacts and can share an origin when assigned distinct URL prefixes (or use separate
domains). HTML and Universal apps are server processes; each needs its own configured bind
address, or a reverse proxy must route distinct hostnames/path prefixes to their processes.
Their current demo defaults may both use port 8080, so starting multiple server demos together
without changing binds will conflict. No production process supervisor, deployment manifest,
or multi-app HTTP host is provided by these examples.

When hosting several apps is in scope, add an explicit deployment configuration mapping each
app to its mode, public hostname or path prefix, artifact/process, bind address, and health
check. Do not infer production routing from Cargo package names or make `run-many` silently
choose ports. Until then, build and deploy each app as its own unit.

## Make targets (simple, per-app)

| Target | Effect |
| --- | --- |
| `make demo` | Run demo-app (bundle + DI + console skeleton) |
| `make html` | Run the default HTML-mode app |
| `make html-all` | Run every registered HTML-mode app |
| `make myfeed` | Legacy alias for `make html` |
| `make spa` | Build demo-spa with Trunk (SPA mode) |
| `make spa-all` | Build every registered SPA app |
| `make ssg` | Build and prerender the default SSG app |
| `make ssg-all` | Build and prerender every registered SSG app |
| `make universal` | Build the frontend and run the Universal backend |
| `make demo-ssg` | Build and prerender demo-ssg (SSG mode) |
| `make demo-universal` | Build the frontend and run demo-universal |

## Phases

| Phase | Delivers | Status |
| --- | --- | --- |
| B | FRONT-MODES.md · demo-html rename · `serenade-html` / `make html` | ✅ done |
| C | demo-monorepo · demo-spa · demo-ui · `serenade-spa` | ✅ done |
| D | demo-universal · demo-front · demo-back · demo-contract · `serenade-universal` | ✅ done |
| E | rangular SSR/hydration guidance · demo-ssg · `serenade-ssg` | ✅ done |
| F | All four modes green in CI · hosting boundary and `run-many` contract documented · handoff current | ✅ docs updated; `run-many` implementation remains future work |

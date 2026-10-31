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
      demo-ssg/             # SSG → per-route HTML + hydrate wasm (after rangular SSG)
    libs/
      demo-ui/              # shared panels/chrome used by demo-spa and demo-ssg
  demo-universal/           # mode Universal (needs a server process)
    apps/
      demo-front/           # hello Leptos+rangular (ultra-simple shop-shaped front)
      demo-back/            # small Serenade listen API (+ serve front assets)
    libs/
      demo-contract/        # shared serde DTO(s) for GET /hello (and similar)
```

### What each folder demos

| Path | Demoes | Does not demo |
| --- | --- | --- |
| `demo-app` | Serenade **bundle**: config packages, Extension, DI service, routes, console commands, event subscriber | HTML site, wasm SPA, SSG, product shop |
| `demo-html` | Mode **HTML**: server-rendered pages, forms/CSRF, Clitorine/Bootstrap (MyFeed) | Leptos, rangular, Trunk dist |
| `demo-monorepo` | Multi-app front workspace: **apps** deployable, **libs** shared; SPA and later SSG without copying UI | Backend API, Universal listen |
| `demo-monorepo/apps/demo-spa` | Mode **SPA**: hello + button, client routes in wasm | Per-route HTML files on disk |
| `demo-monorepo/apps/demo-ssg` | Mode **SSG**: same UI via `demo-ui`, prerendered HTML per route + hydrate | Running Serenade HTTP server |
| `demo-monorepo/libs/demo-ui` | Shared rangular/Leptos UI for spa and ssg | Standalone deploy |
| `demo-universal` | Mode **Universal**: one process (or documented pair) front+back | Full rustashop; flat CDN-only marketing |
| `demo-universal/apps/demo-front` | Hello wasm UI talking to demo-back | SSG export |
| `demo-universal/apps/demo-back` | Boot Serenade kernel, `listen`, `GET /hello` (JSON), serve front assets | HTML demo (demo-html), console tutorial |
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

SSG is blocked until rangular delivers render-to-string + hydrate (see Phase E of plan).

## Mode commands

The HTML and SPA mode commands use the same selection shape: no app name selects that mode's
default app, an app name selects it explicitly, and `--all` selects every app registered for
that mode. The name argument is useful when a mode has multiple registered apps; currently
there is only one HTML app and one SPA app, so naming the current app is equivalent to using
the default command.

| Mode | Default | Named app | All apps | Make |
| --- | --- | --- | --- | --- |
| HTML | `serenade-html` | `serenade-html demo-html` | `serenade-html --all` | `make html` / `make html-all` |
| SPA | `serenade-spa` | `serenade-spa demo-spa` | `serenade-spa --all` | `make spa` / `make spa-all` |

HTML runs the app server (default `127.0.0.1:8080`); `--all` starts all selected HTML apps.
SPA builds with Trunk to `examples/demo-monorepo/apps/demo-spa/dist/`.
Universal builds the `demo-front` Trunk bundle and starts `demo-back` on `127.0.0.1:8080`;
the backend serves `/hello` JSON, frontend assets, and the live-rendered `/cms/hello` page.


## Make targets (simple, per-app)

| Target | Effect |
| --- | --- |
| `make demo` | Run demo-app (bundle + DI + console skeleton) |
| `make html` | Run the default HTML-mode app |
| `make html-all` | Run every registered HTML-mode app |
| `make myfeed` | Legacy alias for `make html` |
| `make spa` | Build demo-spa with Trunk (SPA mode) |
| `make spa-all` | Build every registered SPA app |
| `make universal` | Build the frontend and run the Universal backend |
| `make demo-ssg` | Run demo-ssg (SSG mode, Phase E) |
| `make demo-universal` | Run demo-universal (Universal mode, Phase D) |

## Phases

| Phase | Delivers | Status |
| --- | --- | --- |
| B | FRONT-MODES.md · demo-html rename · `serenade-html` / `make html` | ✅ done |
| C | demo-monorepo · demo-spa · demo-ui · `serenade-spa` | ✅ done |
| D | demo-universal · demo-front · demo-back · demo-contract · `serenade-universal` | ✅ done |
| E | rangular SSG · demo-ssg · `serenade-ssg` | blocked on rangular |
| F | All four modes green in CI · `run-many` docs · handoff to release plan | pending |

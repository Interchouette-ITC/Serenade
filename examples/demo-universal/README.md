# demo-universal

A hello-scale Universal app: the Leptos frontend calls the backend's shared `/hello` JSON
contract, while the backend serves the Trunk bundle and a request-time HTML page at
`/cms/hello`.

From the Serenade repository root, build and run with:

```sh
make universal
```

Open <http://127.0.0.1:8080/>. The `/` route is a client-rendered Leptos SPA. Only
`/cms/hello` is server-rendered: the backend renders a complete document per request from
Rangular-authored document, head, and body templates. Rangular AOT compiles those templates to
Leptos views; Leptos renders the views to the initial HTML, and the browser hydrates the
existing body markup. In-place route buttons update the URL and managed title, description,
canonical URL, and repeatable social-image metadata without a full-page navigation. Use View
Source to inspect the server-rendered response.

The backend reads Trunk's validated `trunk-assets.json` manifest at startup and uses its exact
URLs for the `/cms/hello` bootstrap and WASM bundle, including hashed filenames.
Only scripts marked `data-serenade-universal-bootstrap="true"` qualify as the bootstrap.
Frontend files (favicon, banner, stylesheet) and the SPA home link are resolved under the
manifest `public_url`, so the demo also works with a non-root public URL such as
`serenade-trunk build --public-url /app/`. The server writes that public URL onto the
bootstrap script as `data-serenade-public-url`, and hydration reads it back so client and
server produce the same URLs. The SPA shell uses `<base data-trunk-public-url />` with
relative asset paths. `/cms/hello` and `/hello` remain backend routes at the site root.

To run the backend directly after building the frontend (rather than through `make universal`),
pass both absolute asset paths from the repository root:

```sh
SERENADE_UNIVERSAL_BIND=127.0.0.1:8080 \
SERENADE_TRUNK_DIST_DIR="$PWD/examples/demo-universal/apps/demo-front/dist" \
SERENADE_TRUNK_ASSET_MANIFEST="$PWD/examples/demo-universal/apps/demo-front/dist/trunk-assets.json" \
cargo +stable run --manifest-path examples/demo-universal/Cargo.toml -p serenade-demo-back --locked
```

SSR renders the route on each request. SSG instead renders declared routes during the build and
writes static HTML files. AOT compiles templates to view code; by itself it does not render
route HTML or determine whether the app uses SSR or SSG. The existing Leptos SSG demo and
Rust.Contact are unchanged; neither is migrated by this example. The `/hello` API and the
Universal frontend's favicon under the public URL (`/favicon.ico` by default) remain available. Set
`SERENADE_UNIVERSAL_BIND` to change the backend listen address.

Head composition uses Serenade's generic Leptos head lifecycle, not Rangular-specific behavior:
Rangular supplies the head view, while `ManagedHead` and its keyed metadata components manage
route updates. See the [generic head extension point](../../docs-dev/VIEW.md#generic-head-extension-point).
On hydration, only metadata marked `data-serenade-head="ssr"` is removed and remounted.
Leptos reuses the existing title node; unrelated head nodes and the bootstrap script are retained.

The full product reference is [rustashop](../../rustashop); this demo stays intentionally tiny.

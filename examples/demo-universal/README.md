# demo-universal

A hello-scale Universal app: the Leptos front calls the backend's shared `/hello` JSON contract,
and the backend serves the Trunk bundle plus a live HTML page at `/cms/hello`.

From the Serenade repository root, build and run with:

```sh
make universal
```

Open <http://127.0.0.1:8080/>. The `/cms/hello` route renders the shared Leptos `HelloUi`
component on each backend request. Its document shell and visible UI are Leptos views, not
hand-authored HTML strings. Use View Source to inspect the server-rendered page; this is a live
SSR response, not an SSG output. The rendered component comes from
`examples/demo-monorepo/libs/demo-ui`, so the same UI is reused by the SPA and SSG examples.
The page hydrates that same component in the browser, so its counter button is interactive
after the wasm bundle loads. The Universal frontend also serves the Serenade favicon at
`/favicon.ico`. Set `SERENADE_UNIVERSAL_BIND` to change the backend listen address.

The full product reference is [rustashop](../../rustashop); this demo stays intentionally tiny.

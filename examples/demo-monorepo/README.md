# demo-monorepo

This front-end workspace keeps deployable apps under `apps/` and reusable UI under `libs/`.
The SPA and the future SSG app share `libs/demo-ui`; the SSG app is intentionally not present
until rangular supports render-to-string and hydration in Phase E.

## SPA

`apps/demo-spa` is a small Leptos client-rendered app. Its shared `HelloUi` component shows a
title and a button with a click counter. Trunk writes the browser bundle to
`apps/demo-spa/dist/`.

From the Serenade repository root:

```sh
make spa
# equivalent to:
cargo run -p serenade-cli --bin serenade-spa
```

The CLI defaults to `demo-spa`. `make spa-all` or `serenade-spa --all` builds every app
registered with `mode = "spa"`; currently, that is only `demo-spa`.

## SSG

The planned `apps/demo-ssg` will reuse `libs/demo-ui` to demonstrate prerendered route HTML and
hydration. It is not ready until Phase E; this workspace currently contains no SSG app.

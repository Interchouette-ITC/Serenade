# demo-monorepo

This front-end workspace keeps deployable apps under `apps/` and reusable UI under `libs/`.
The SPA and SSG app share `libs/demo-ui`; the SSG app renders that Leptos component to static
HTML and hydrates it in the browser.

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

`apps/demo-ssg` uses the same `HelloUi` component as the SPA. Its native SSR build prerenders
two routes to `dist/index.html` and `dist/about/index.html`; its wasm build hydrates those
existing pages and keeps the button interactive. The generated source HTML is directly visible
in those files and in the browser's View Source.

From the Serenade repository root:

```sh
make ssg
# equivalent to:
cargo run -p serenade-cli --bin serenade-ssg
```

The CLI defaults to `demo-ssg`; `make ssg-all` or `serenade-ssg --all` builds every app
registered with `mode = "ssg"`.

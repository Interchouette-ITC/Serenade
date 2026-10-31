# demo-universal

A hello-scale Universal app: the Leptos front calls the backend's shared `/hello` JSON contract,
and the backend serves the Trunk bundle plus a live HTML page at `/cms/hello`.

From the Serenade repository root, build and run with:

```sh
make universal
```

Open <http://127.0.0.1:8080/>. The `/cms/hello` route is server-rendered per request; it is
not an SSG output. Set `SERENADE_UNIVERSAL_BIND` to change the backend listen address.

The full product reference is [rustashop](../../rustashop); this demo stays intentionally tiny.

# Admin CRUD (`serenade-admin`)

Optional EasyAdmin-shaped Admin CRUD helpers live in **`serenade-admin`** ([#177](https://github.com/Interchouette-ITC/Serenade/issues/177), [#178](https://github.com/Interchouette-ITC/Serenade/issues/178)).

**Not** part of the kernel or `FrameworkBundle`. Apps opt in with `AdminBundle` / `AdminExtension`.

Core stance (no Symfony 1 generator in FrameworkBundle): [KERNEL.md](KERNEL.md#admin-back-office), [#122](https://github.com/Interchouette-ITC/Serenade/issues/122).

## What ships today

| Piece | Status |
| --- | --- |
| `AdminResource` / `AdminField` / `AdminRow` | Yes |
| `AdminRegistry` + DI `admin.registry` | Yes |
| List + show route registration | Yes |
| List + show HTML helpers (escaped) | Yes |
| New / edit / delete + Form + CSRF | Next ([#179](https://github.com/Interchouette-ITC/Serenade/issues/179)) |
| MyFeed / recipe dogfood | Later ([#180](https://github.com/Interchouette-ITC/Serenade/issues/180)) |

## Ownership

| Piece | Bundle | App |
| --- | --- | --- |
| Resource field map + path prefix | Yes | Declares resources |
| List / show HTML orchestration | Yes | Supplies `AdminRow` data |
| Persist / query | Calls app hooks (forms slice) | Repositories |
| Auto-wire into FrameworkBundle | **No** | Opt-in extension |

## Example

```rust
use serenade_admin::{
    AdminField, AdminRegistry, AdminResource, AdminRow, register_admin_routes, render_list_html,
};
use serenade_http::RouteCollection;

let mut registry = AdminRegistry::new();
registry.register(
    AdminResource::new("product", "/admin/products")
        .list_fields([AdminField::named("name"), AdminField::new("price", "Price")]),
)?;

let mut routes = RouteCollection::new();
register_admin_routes(&mut routes, &registry)?;

let html = render_list_html(
    registry.get("product").expect("resource"),
    &[AdminRow::new("1").with("name", "Mug").with("price", "12")],
);
assert!(html.contains("Mug"));
```

Wire DI with `AdminExtension` next to `FrameworkExtension`, then resolve `admin.registry` and register resources before adding routes.

## Related

- Epic: [#177](https://github.com/Interchouette-ITC/Serenade/issues/177)
- Forms CRUD: [#179](https://github.com/Interchouette-ITC/Serenade/issues/179)
- Dogfood: [#180](https://github.com/Interchouette-ITC/Serenade/issues/180)

# Admin CRUD (`serenade-admin`)

Optional EasyAdmin-shaped Admin CRUD helpers live in **`serenade-admin`** ([#177](https://github.com/Interchouette-ITC/Serenade/issues/177)).

**Not** part of the kernel or `FrameworkBundle`. Apps opt in with `AdminBundle` / `AdminExtension` and a Flex recipe.

Core stance (no Symfony 1 generator in FrameworkBundle): [KERNEL.md](KERNEL.md#admin-back-office), [#122](https://github.com/Interchouette-ITC/Serenade/issues/122).

## What ships today

| Piece | Status |
| --- | --- |
| `AdminResource` / `AdminField` / `AdminRow` | Yes |
| `AdminRegistry` + DI `admin.registry` | Yes |
| List / show / new / create / edit / update / delete routes | Yes |
| List + show HTML helpers | Yes |
| New / edit / delete forms + CSRF (`serenade-form`) | Yes |
| `AdminResourceHandler` persist hooks | Yes (app-owned) |
| Flex recipe `admin` → `config/packages/admin.toml` | Yes |
| MyFeed dogfood (categories) | Yes ([#180](https://github.com/Interchouette-ITC/Serenade/issues/180)) |

## Recipe

```bash
make serenade ARGS='recipe apply admin --root /path/to/app --no-cargo'
# or: cargo add serenade-admin && copy the sample TOML
```

Sample package file:

```toml
[admin]
# Optional Admin CRUD package. Not part of FrameworkBundle.
enabled = true
```

Register in the app:

1. Add `AdminBundle` to the bundle list and load `AdminExtension` with `build_container`.
2. Resolve `admin.registry`, `register` one or more `AdminResource` values.
3. Call `register_admin_routes` on the app `RouteCollection`.
4. Implement `AdminResourceHandler` and bind HTML / Form POSTs in your HTTP layer.

Resources are declared in **application code** (not auto-generated from the TOML). The package file marks the opt-in and carries future config.

## Ownership

| Piece | Bundle | App |
| --- | --- | --- |
| Resource field map + path prefix | Yes | Declares resources |
| List / show / form HTML | Yes | Supplies `AdminRow` / calls handlers |
| Persist / query | `AdminResourceHandler` | Repositories |
| Auto-wire into FrameworkBundle | **No** | Opt-in extension |

## Forms

```rust
use serenade_admin::{
    AdminField, AdminResource, AdminRow, render_edit_form_html, render_new_form_html,
};
use serenade_security::HmacCsrfTokenManager;

let resource = AdminResource::new("product", "/admin/products")
    .list_fields([AdminField::named("name")])
    .form_fields([AdminField::named("name")]);
let mgr = HmacCsrfTokenManager::new(b"app-secret-at-least-32-bytes-long!!");

let new_html = render_new_form_html(&resource, &mgr)?;
let edit_html = render_edit_form_html(
    &resource,
    &AdminRow::new("1").with("name", "Mug"),
    &mgr,
)?;
```

Bind POSTs with `Form::handle_request` + CSRF manager, then call `AdminResourceHandler::create` / `update` / `delete`.

## MyFeed dogfood

[`examples/MyFeed`](../examples/MyFeed) wires one resource: **categories**.

| Surface | Role |
| --- | --- |
| `/admin` | Moderation queue + category list panel (`render_list_html`) |
| `/admin/categories` | Full CRUD list |
| `/admin/categories/new` | Create (CSRF form) |
| `/admin/categories/{id}` | Show / edit / delete |

Sample package: `examples/MyFeed/config/packages/admin.toml`. Handlers live in `examples/MyFeed/src/admin_crud.rs` (`CategoryHandler`).

See also [MYFEED.md](MYFEED.md), [FORMS.md](FORMS.md), [RECIPES.md](RECIPES.md).

## Related

- Epic: [#177](https://github.com/Interchouette-ITC/Serenade/issues/177)
- Forms primitives: [FORMS.md](FORMS.md)

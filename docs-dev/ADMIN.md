# Admin CRUD (`serenade-admin`)

Optional EasyAdmin-shaped Admin CRUD helpers live in **`serenade-admin`** ([#177](https://github.com/Interchouette-ITC/Serenade/issues/177)).

**Not** part of the kernel or `FrameworkBundle`. Apps opt in with `AdminBundle` / `AdminExtension`.

Core stance (no Symfony 1 generator in FrameworkBundle): [KERNEL.md](KERNEL.md#admin-back-office), [#122](https://github.com/Interchouette-ITC/Serenade/issues/122).

## What ships today

| Piece | Status |
| --- | --- |
| `AdminResource` / `AdminField` / `AdminRow` | Yes |
| `AdminRegistry` + DI `admin.registry` | Yes |
| List / show / new / create / edit / update / delete routes | Yes ([#178](https://github.com/Interchouette-ITC/Serenade/issues/178), [#179](https://github.com/Interchouette-ITC/Serenade/issues/179)) |
| List + show HTML helpers | Yes |
| New / edit / delete forms + CSRF (`serenade-form`) | Yes ([#179](https://github.com/Interchouette-ITC/Serenade/issues/179)) |
| `AdminResourceHandler` persist hooks | Yes (app-owned) |
| MyFeed / recipe dogfood | Later ([#180](https://github.com/Interchouette-ITC/Serenade/issues/180)) |

## Ownership

| Piece | Bundle | App |
| --- | --- | --- |
| Resource field map + path prefix | Yes | Declares resources |
| List / show / form HTML | Yes | Supplies `AdminRow` / calls handlers |
| Persist / query | [`AdminResourceHandler`](https://docs.rs/serenade-admin) | Repositories |
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

## Related

- Epic: [#177](https://github.com/Interchouette-ITC/Serenade/issues/177)
- Dogfood: [#180](https://github.com/Interchouette-ITC/Serenade/issues/180)
- Forms primitives: [FORMS.md](FORMS.md)

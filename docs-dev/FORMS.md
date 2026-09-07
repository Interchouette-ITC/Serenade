# Forms

HTML form bind, CSRF (default on), and XSS-safe render helpers.

Apps declare fields and constraints. **Serenade owns** CSRF validation and HTML escaping so MyFeed / RustaShop stay thin.

## Crate

**`serenade-form`** ([#110](https://github.com/Interchouette-ITC/Serenade/issues/110))

| Piece | Role |
| --- | --- |
| `Form` / `FormBuilder` | Build fields, bind POST body, validate via `serenade-validator` |
| CSRF (default **on**) | Hidden `_token` field; [`HmacCsrfTokenManager`](SECURITY.md) |
| `escape_html` / `escape_attr` | Twig autoescape analogue for render |
| `parse_urlencoded` | `application/x-www-form-urlencoded` body → map |
| `Form::render` | Escaped `<form>` markup including CSRF when enabled |

## Flow

```text
GET  → Form::prepare_csrf(manager) → Form::render() → HTML
POST → Form::handle_request(request, manager)  // CSRF check
     → Form::is_valid() / validate(&validator)
     → read Form::data() / get("field")
```

## CSRF

Tokens are HMAC-SHA256 signed and **stateless** (no session store required for v0). Secret is app-owned and must be stable across requests.

Field name: `_token` (`CSRF_FIELD_NAME`), same habit as Symfony.

Disable only with `Form::builder(…).csrf(false)` when you have a deliberate non-browser client (prefer keeping CSRF on for HTML).

## XSS

`Form::render` escapes field values and attributes. Prefer `escape_html` for any user text you concatenate into HTML outside the form helper.

## Related

- CSRF manager API: [SECURITY.md](SECURITY.md)
- Validator constraints: [KERNEL.md](KERNEL.md) (Validator)
- Beginner demo: MyFeed (`examples/MyFeed`, epic #112) depends on this crate

# Forms

HTML form bind, CSRF (default on), XSS-safe theme helpers, and multipart file uploads.

Apps declare fields, kinds, and constraints. **Serenade owns** CSRF validation and HTML escaping so apps stay thin.

## Crate

**`serenade-form`** ([#110](https://github.com/Interchouette-ITC/Serenade/issues/110), [#278](https://github.com/Interchouette-ITC/Serenade/issues/278), [#279](https://github.com/Interchouette-ITC/Serenade/issues/279))

| Piece | Role |
| ----- | ---- |
| `Form` / `FormBuilder` | Build fields, bind POST body, validate via `serenade-validator` |
| `FieldKind` / `FieldConfig` | Text, Textarea, Hidden, Password, Checkbox, Choice, Number, File |
| Compound / collection | Dotted names (`address.street`) and indexed keys (`items[0][name]`) |
| `DataTransformer` | View ↔ model string transforms on bind/render |
| CSRF (default **on**) | Hidden `_token` field; [`HmacCsrfTokenManager`](SECURITY.md) |
| Theme helpers | `form_label` / `form_widget` / `form_errors` / `form_row` / `form_csrf` (escaped) |
| `escape_html` / `escape_attr` | Twig autoescape analogue for custom markup |
| `parse_urlencoded` / `parse_multipart` | Urlencoded and `multipart/form-data` bodies |
| `UploadedFile` | Filename + bytes or tempfile for `FieldKind::File` |
| `Form::render` | Full `<form>` including CSRF when enabled; multipart `enctype` when a File field exists |

## Flow

```text
GET  → Form::prepare_csrf(manager) → Form::render() / form_row(…) → HTML
POST → Form::handle_request(request, manager)  // CSRF + urlencoded or multipart
     → Form::is_valid() / validate(&validator)
     → read Form::data() / get("field") / file("upload")
```

## Symfony Form concept map

Serenade copies the **ergonomics** that matter for HTML apps. It does **not** port the PHP Form component tree.

| Symfony idea | In Serenade | Status |
| ------------ | ----------- | ------ |
| `FormBuilder` / `createForm` | `Form::builder` / `FormBuilder` | Copy (KISS) |
| CSRF (`_token`) | Enabled by default; `form_csrf` / `Form::render` | Copy |
| Field types (`TextType`, …) | `FieldKind` + typed `*Options` structs | Copy (no PHP class hierarchy) |
| `ChoiceType` | `FieldConfig::Choice` / `ChoiceOptions` | Copy |
| `FileType` / `UploadedFile` | `FieldConfig::File` / `UploadedFile` | Copy (limits below) |
| Compound / embedded forms | `FormBuilder::compound` (dotted names) | Copy |
| `CollectionType` | `FormBuilder::collection` + prototype helpers | Copy (static count at build) |
| Data transformers | `DataTransformer` trait | Copy |
| Form themes / `form_row` | `form_row` / `form_widget` / `form_label` / `form_errors` | Copy (HTML strings, not Twig) |
| `OptionsResolver` | Typed options structs on each kind | Refuse (no resolver crate) |
| `AbstractType` inheritance | - | Refuse |
| Twig form themes | - | Refuse |
| EasyAdmin / Sonata grids | - | Refuse |
| Every PrestaShop FormType | - | Refuse |

## CSRF

Tokens are HMAC-SHA256 signed and **stateless** (no session store required for v0). Secret is app-owned and must be stable across requests.

Field name: `_token` (`CSRF_FIELD_NAME`), same habit as Symfony.

Disable only with `Form::builder(…).csrf(false)` when you have a deliberate non-browser client (prefer keeping CSRF on for HTML).

## XSS

`Form::render` and the theme helpers escape field values, labels, attributes, and error messages. Prefer `escape_html` for any user text you concatenate into HTML outside these helpers.

## Multipart and File

`Form::handle_request` reads `Content-Type`:

- `application/x-www-form-urlencoded` (default when unset) → text fields only
- `multipart/form-data` → text fields + file parts

`FieldKind::File` requires multipart. `Form::render` sets `enctype="multipart/form-data"` when the form declares a File field.

### Limits (defaults)

| Limit | Default | Override |
| ----- | ------- | -------- |
| Per-file size | **2 MiB** (`DEFAULT_MAX_FILE_SIZE`) | `FileOptions.max_size` |
| Multipart body | **8 MiB** (`DEFAULT_MAX_MULTIPART_BODY`) | `parse_multipart(…, max_body)` |
| MIME allow-list | empty = any | `FileOptions.mime_types` (supports `image/*`) |
| Storage | in-memory bytes | `FileOptions.storage = FileStorage::TempFile` |

Apps should still enforce HTTP body limits at the server adapter. Empty browser file inputs (`filename=""` with empty body) are treated as absent uploads.

```rust
use serenade_form::{FieldConfig, FileOptions, Form, FormStatus};

let mut form = Form::builder("avatar")
    .field_config(
        "photo",
        FieldConfig::File(FileOptions {
            accept: Some("image/*".into()),
            max_size: 200 * 1024,
            mime_types: vec!["image/png".into(), "image/jpeg".into()],
            ..FileOptions::default()
        }),
        vec![],
    )
    .build();
// prepare_csrf → render on GET; handle_request on POST; form.file("photo")
```

## Theme helpers

| Helper | Output |
| ------ | ------ |
| `form_label` | Escaped `<label for="…">` (empty for Hidden / expanded Choice) |
| `form_widget` | Escaped control only |
| `form_errors` | Escaped `<ul class="form-errors">` for that field path |
| `form_row` | `div.form-row` wrapping label + widget + errors |
| `form_csrf` | CSRF hidden input when enabled |

## Related

- CSRF manager API: [SECURITY.md](SECURITY.md)
- Validator constraints: [KERNEL.md](KERNEL.md) (Validator)
- HTML apps: see `examples/MyFeed` (depends on this crate)

# serenade-form

HTML forms: bind request (urlencoded or multipart), CSRF by default, XSS-safe
theme helpers (`form_row`, `form_widget`, …), and `FieldKind::File` uploads.

Apps declare fields and constraints; Serenade owns CSRF and XSS-safe escaping.
See `docs-dev/FORMS.md` and `docs-dev/SECURITY.md`.

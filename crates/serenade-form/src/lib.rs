//! HTML form component: bind request, CSRF by default, XSS-safe render helpers.
//!
//! Apps declare fields, kinds, and constraints; Serenade owns CSRF validation and
//! XSS-safe escaping. Theme helpers (`form_row`, `form_widget`, …) and multipart
//! uploads (`FieldKind::File`) are part of this crate. See `docs-dev/FORMS.md`.

mod collection;
mod error;
mod escape;
mod file;
mod form;
mod kind;
mod multipart;
mod parse;
mod render;
mod theme;
mod transformer;

pub use collection::{
    COLLECTION_PROTOTYPE_INDEX, CollectionPrototypeMeta, NestedData, collection_field_name,
    collection_indices, collection_prototype_field_name, unflatten_form_data,
};
pub use error::FormError;
pub use escape::{escape_attr, escape_html};
pub use file::{FileStorage, UploadedFile};
pub use form::{CollectionEntryBuilder, CompoundBuilder, Field, Form, FormBuilder, FormStatus};
pub use kind::{
    CheckboxOptions, ChoiceEntry, ChoiceOptions, DEFAULT_MAX_FILE_SIZE, DEFAULT_MAX_MULTIPART_BODY,
    FieldConfig, FieldKind, FileOptions, HiddenOptions, NumberOptions, PasswordOptions,
    TextOptions, TextareaOptions,
};
pub use multipart::{
    MultipartData, is_multipart, multipart_boundary, parse_form_body, parse_multipart,
};
pub use parse::{parse_urlencoded, parse_urlencoded_multi};
pub use render::RenderedForm;
pub use serenade_security::CSRF_FIELD_NAME;
pub use theme::{form_csrf, form_errors, form_label, form_row, form_widget};
pub use transformer::{
    BoolToStringTransformer, ChoiceToValueTransformer, DataTransformer, I64ToStringTransformer,
    join_multi, split_multi,
};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;

//! Field kinds and typed option structs.

use crate::file::FileStorage;

/// Default per-file size limit enforced when [`FileOptions::max_size`] is unset (2 MiB).
pub const DEFAULT_MAX_FILE_SIZE: u64 = 2 * 1024 * 1024;

/// Default multipart body size limit for [`crate::parse_multipart`] (8 MiB).
pub const DEFAULT_MAX_MULTIPART_BODY: u64 = 8 * 1024 * 1024;

/// HTML-oriented field kind used for bind and render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Single-line text (`input type="text"`).
    Text,
    /// Multi-line text (`textarea`).
    Textarea,
    /// Hidden input.
    Hidden,
    /// Password input (value still stored as a string).
    Password,
    /// Checkbox (`input type="checkbox"`).
    Checkbox,
    /// Single or multi select / radio group (see [`ChoiceOptions::multiple`]).
    Choice,
    /// Numeric input (`input type="number"`).
    Number,
    /// File upload (`input type="file"`); bind via multipart only.
    File,
}

/// Options for [`FieldKind::Text`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextOptions {
    /// Optional `placeholder` attribute.
    pub placeholder: Option<String>,
}

/// Options for [`FieldKind::Textarea`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextareaOptions {
    /// Optional `placeholder` attribute.
    pub placeholder: Option<String>,
    /// Optional `rows` attribute.
    pub rows: Option<u32>,
}

/// Options for [`FieldKind::Hidden`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiddenOptions {}

/// Options for [`FieldKind::Password`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PasswordOptions {
    /// Optional `placeholder` attribute.
    pub placeholder: Option<String>,
}

/// Options for [`FieldKind::Checkbox`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxOptions {
    /// Submitted value when the box is checked (default `"1"`).
    pub checked_value: String,
}

impl Default for CheckboxOptions {
    fn default() -> Self {
        Self {
            checked_value: String::from("1"),
        }
    }
}

/// One choice entry: submitted value and display label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceEntry {
    /// Value posted when this choice is selected.
    pub value: String,
    /// Human-readable label.
    pub label: String,
}

impl ChoiceEntry {
    /// Builds a choice with the same string for value and label.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            label: value.clone(),
            value,
        }
    }

    /// Builds a choice with distinct value and label.
    #[must_use]
    pub fn with_label(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// Options for [`FieldKind::Choice`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChoiceOptions {
    /// Allowed choices.
    pub choices: Vec<ChoiceEntry>,
    /// When `true`, multiple values may be selected (stored space-separated in [`crate::Field::value`]).
    pub multiple: bool,
    /// When `true`, prefer expanded widgets (radio/checkbox list) over `<select>` in render.
    pub expanded: bool,
}

/// Options for [`FieldKind::Number`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NumberOptions {
    /// Optional HTML `min`.
    pub min: Option<i64>,
    /// Optional HTML `max`.
    pub max: Option<i64>,
    /// Optional HTML `step` (integer step).
    pub step: Option<i64>,
}

/// Options for [`FieldKind::File`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOptions {
    /// Optional HTML `accept` attribute (for example `image/*`).
    pub accept: Option<String>,
    /// Maximum upload size in bytes (default [`DEFAULT_MAX_FILE_SIZE`]).
    pub max_size: u64,
    /// Allowed MIME types; empty means any type is accepted.
    pub mime_types: Vec<String>,
    /// Whether the payload stays in memory or spills to a tempfile.
    pub storage: FileStorage,
}

impl Default for FileOptions {
    fn default() -> Self {
        Self {
            accept: None,
            max_size: DEFAULT_MAX_FILE_SIZE,
            mime_types: Vec::new(),
            storage: FileStorage::Memory,
        }
    }
}

/// Typed configuration for a field (kind + options).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldConfig {
    /// [`FieldKind::Text`].
    Text(TextOptions),
    /// [`FieldKind::Textarea`].
    Textarea(TextareaOptions),
    /// [`FieldKind::Hidden`].
    Hidden(HiddenOptions),
    /// [`FieldKind::Password`].
    Password(PasswordOptions),
    /// [`FieldKind::Checkbox`].
    Checkbox(CheckboxOptions),
    /// [`FieldKind::Choice`].
    Choice(ChoiceOptions),
    /// [`FieldKind::Number`].
    Number(NumberOptions),
    /// [`FieldKind::File`].
    File(FileOptions),
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self::Text(TextOptions::default())
    }
}

impl FieldConfig {
    /// Returns the [`FieldKind`] for this config.
    #[must_use]
    pub const fn kind(&self) -> FieldKind {
        match self {
            Self::Text(_) => FieldKind::Text,
            Self::Textarea(_) => FieldKind::Textarea,
            Self::Hidden(_) => FieldKind::Hidden,
            Self::Password(_) => FieldKind::Password,
            Self::Checkbox(_) => FieldKind::Checkbox,
            Self::Choice(_) => FieldKind::Choice,
            Self::Number(_) => FieldKind::Number,
            Self::File(_) => FieldKind::File,
        }
    }

    /// Returns choice options when this is a choice field.
    #[must_use]
    pub const fn as_choice(&self) -> Option<&ChoiceOptions> {
        match self {
            Self::Choice(options) => Some(options),
            _ => None,
        }
    }

    /// Returns checkbox options when this is a checkbox field.
    #[must_use]
    pub const fn as_checkbox(&self) -> Option<&CheckboxOptions> {
        match self {
            Self::Checkbox(options) => Some(options),
            _ => None,
        }
    }

    /// Returns number options when this is a number field.
    #[must_use]
    pub const fn as_number(&self) -> Option<&NumberOptions> {
        match self {
            Self::Number(options) => Some(options),
            _ => None,
        }
    }

    /// Returns file options when this is a file field.
    #[must_use]
    pub const fn as_file(&self) -> Option<&FileOptions> {
        match self {
            Self::File(options) => Some(options),
            _ => None,
        }
    }
}

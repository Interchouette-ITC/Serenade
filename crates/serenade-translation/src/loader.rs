//! Pluggable catalogue loaders (TOML first, JSON allowed).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{Locale, MessageCatalogue, TranslationError};

/// Loads a catalogue document into a domain map (`id` → template).
///
/// Apps and third-party crates implement this for XLIFF, Gettext, Fluent, etc.
pub trait CatalogueLoader: Send + Sync {
    /// Format id used in file discovery (`toml`, `json`, …).
    fn format(&self) -> &'static str;

    /// Parses `bytes` into message id → template.
    ///
    /// # Errors
    ///
    /// Returns [`TranslationError::Load`] when the payload is invalid.
    fn parse(&self, bytes: &[u8], path: &Path)
    -> Result<HashMap<String, String>, TranslationError>;
}

/// TOML catalogue: flat string table or nested `{ messages = { … } }`.
#[derive(Clone, Copy, Debug, Default)]
pub struct TomlCatalogueLoader;

impl CatalogueLoader for TomlCatalogueLoader {
    fn format(&self) -> &'static str {
        "toml"
    }

    fn parse(
        &self,
        bytes: &[u8],
        path: &Path,
    ) -> Result<HashMap<String, String>, TranslationError> {
        let text = std::str::from_utf8(bytes).map_err(|err| TranslationError::Load {
            path: path.to_path_buf(),
            detail: err.to_string(),
        })?;
        let value: toml::Value = toml::from_str(text).map_err(|err| TranslationError::Load {
            path: path.to_path_buf(),
            detail: err.to_string(),
        })?;
        flatten_toml(&value, path)
    }
}

/// JSON catalogue: object of string values, or `{ "messages": { … } }`.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsonCatalogueLoader;

impl CatalogueLoader for JsonCatalogueLoader {
    fn format(&self) -> &'static str {
        "json"
    }

    fn parse(
        &self,
        bytes: &[u8],
        path: &Path,
    ) -> Result<HashMap<String, String>, TranslationError> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|err| TranslationError::Load {
                path: path.to_path_buf(),
                detail: err.to_string(),
            })?;
        flatten_json(&value, path)
    }
}

fn flatten_toml(
    value: &toml::Value,
    path: &Path,
) -> Result<HashMap<String, String>, TranslationError> {
    match value {
        toml::Value::Table(table) => table
            .get("messages")
            .and_then(toml::Value::as_table)
            .map_or_else(
                || table_to_strings(table, path),
                |messages| table_to_strings(messages, path),
            ),
        _ => Err(TranslationError::Load {
            path: path.to_path_buf(),
            detail: "root must be a table of string messages".into(),
        }),
    }
}

fn table_to_strings(
    table: &toml::map::Map<String, toml::Value>,
    path: &Path,
) -> Result<HashMap<String, String>, TranslationError> {
    let mut out = HashMap::new();
    for (key, value) in table {
        match value {
            toml::Value::String(text) => {
                out.insert(key.clone(), text.clone());
            }
            toml::Value::Integer(n) => {
                out.insert(key.clone(), n.to_string());
            }
            toml::Value::Float(n) => {
                out.insert(key.clone(), n.to_string());
            }
            toml::Value::Boolean(b) => {
                out.insert(key.clone(), b.to_string());
            }
            _ => {
                return Err(TranslationError::Load {
                    path: path.to_path_buf(),
                    detail: format!("message `{key}` must be a scalar string"),
                });
            }
        }
    }
    Ok(out)
}

fn flatten_json(
    value: &serde_json::Value,
    path: &Path,
) -> Result<HashMap<String, String>, TranslationError> {
    let object = match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Object(messages)) = map.get("messages") {
                messages
            } else {
                map
            }
        }
        _ => {
            return Err(TranslationError::Load {
                path: path.to_path_buf(),
                detail: "root must be a JSON object of string messages".into(),
            });
        }
    };
    let mut out = HashMap::new();
    for (key, value) in object {
        let text = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => {
                return Err(TranslationError::Load {
                    path: path.to_path_buf(),
                    detail: format!("message `{key}` must be a scalar string"),
                });
            }
        };
        out.insert(key.clone(), text);
    }
    Ok(out)
}

/// Parsed catalogue file name: `{domain}.{locale}.{format}` or `{domain}+intl-icu.{locale}.{format}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogueFileName {
    /// Message domain (may include `+intl-icu`).
    pub domain: String,
    /// Locale tag from the file name.
    pub locale: Locale,
    /// Extension without dot (`toml`, `json`).
    pub format: String,
}

impl CatalogueFileName {
    /// Parses `messages.en.toml` or `messages+intl-icu.fr_FR.json`.
    ///
    /// Underscores in the locale segment become hyphens (`fr_FR` → `fr-FR`).
    #[must_use]
    pub fn parse(file_name: &str) -> Option<Self> {
        let path = Path::new(file_name);
        let stem = path.file_stem()?.to_str()?;
        let format = path.extension()?.to_str()?.to_ascii_lowercase();
        let (domain, locale_raw) = stem.rsplit_once('.')?;
        let locale_raw = locale_raw.replace('_', "-");
        let locale = Locale::new(locale_raw).ok()?;
        Some(Self {
            domain: domain.to_owned(),
            locale,
            format,
        })
    }
}

/// Loads every supported file under `dir` into catalogues keyed by locale.
///
/// # Errors
///
/// Returns [`TranslationError`] when the directory cannot be read or a file fails
/// to parse.
pub fn load_directory(
    dir: impl AsRef<Path>,
    loaders: &[&dyn CatalogueLoader],
) -> Result<HashMap<Locale, MessageCatalogue>, TranslationError> {
    let dir = dir.as_ref();
    let entries = fs::read_dir(dir).map_err(|err| TranslationError::Directory {
        path: dir.to_path_buf(),
        detail: err.to_string(),
    })?;

    let mut by_locale: HashMap<Locale, MessageCatalogue> = HashMap::new();

    for entry in entries {
        let entry = entry.map_err(|err| TranslationError::Directory {
            path: dir.to_path_buf(),
            detail: err.to_string(),
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(meta) = CatalogueFileName::parse(name) else {
            continue;
        };
        let Some(loader) = loaders.iter().find(|loader| loader.format() == meta.format) else {
            continue;
        };
        let bytes = fs::read(&path).map_err(|err| TranslationError::Load {
            path: path.clone(),
            detail: err.to_string(),
        })?;
        let messages = loader.parse(&bytes, &path)?;
        let catalogue = by_locale
            .entry(meta.locale.clone())
            .or_insert_with(|| MessageCatalogue::new(meta.locale.clone()));
        for (id, text) in messages {
            catalogue.set(meta.domain.clone(), id, text);
        }
    }

    Ok(by_locale)
}

/// Loads catalogues from one or more directories, merging in order (later wins).
///
/// # Errors
///
/// Propagates [`load_directory`] failures.
pub fn load_paths(
    paths: &[PathBuf],
    loaders: &[&dyn CatalogueLoader],
) -> Result<HashMap<Locale, MessageCatalogue>, TranslationError> {
    let mut merged: HashMap<Locale, MessageCatalogue> = HashMap::new();
    for path in paths {
        let batch = load_directory(path, loaders)?;
        for (locale, catalogue) in batch {
            merged
                .entry(locale.clone())
                .or_insert_with(|| MessageCatalogue::new(locale))
                .replace_catalogue(&catalogue);
        }
    }
    Ok(merged)
}

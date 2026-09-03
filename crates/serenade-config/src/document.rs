//! Config document load, merge, interpolation, and parameter flattening.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value};
use serenade_di::ParameterBag;

use crate::ConfigError;

/// Layered configuration document (JSON object internally).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    value: Value,
}

impl Config {
    /// Empty mapping.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            value: Value::Object(Map::new()),
        }
    }

    /// Parses a YAML mapping.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when YAML is invalid or the root is not a mapping.
    pub fn from_yaml(text: &str) -> Result<Self, ConfigError> {
        let yaml: serde_yaml::Value = serde_yaml::from_str(text)?;
        Self::from_value(serde_json::to_value(yaml)?)
    }

    /// Parses a TOML mapping.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when TOML is invalid or the root is not a mapping.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let table: toml::Table = text.parse()?;
        Self::from_value(serde_json::to_value(table)?)
    }

    /// Loads a `.yaml` / `.yml` / `.toml` file.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] on IO, parse, or format errors.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("yaml" | "yml") => Self::from_yaml(&text),
            Some("toml") => Self::from_toml(&text),
            _ => Err(ConfigError::UnsupportedFormat {
                path: path.to_path_buf(),
            }),
        }
    }

    /// Deep-merges `overlay` on top of `self`. Mappings merge; other values replace.
    #[must_use]
    pub fn merged(&self, overlay: &Self) -> Self {
        Self {
            value: merge_values(self.value.clone(), overlay.value.clone()),
        }
    }

    /// Replaces `${VAR}` and `${VAR:-default}` in string values.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::MissingEnvironment`] when a variable has no default and is unset.
    pub fn interpolate_env(&self) -> Result<Self, ConfigError> {
        Ok(Self {
            value: interpolate_value(self.value.clone())?,
        })
    }

    /// Flattened dotted keys for the DI [`ParameterBag`].
    #[must_use]
    pub fn parameters(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        flatten("", &self.value, &mut out);
        out
    }

    /// Copies flattened parameters into `bag`.
    pub fn apply_to(&self, bag: &mut ParameterBag) {
        for (key, value) in self.parameters() {
            bag.set(key, value);
        }
    }

    /// Root JSON value (mapping).
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }

    fn from_value(value: Value) -> Result<Self, ConfigError> {
        if value.is_object() {
            Ok(Self { value })
        } else {
            Err(ConfigError::InvalidRoot)
        }
    }
}

fn merge_values(base: Value, overlay: Value) -> Value {
    match (base, overlay) {
        (Value::Object(mut base_map), Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                let merged = match base_map.remove(&key) {
                    Some(existing) => merge_values(existing, overlay_value),
                    None => overlay_value,
                };
                base_map.insert(key, merged);
            }
            Value::Object(base_map)
        }
        (_, overlay) => overlay,
    }
}

fn interpolate_value(value: Value) -> Result<Value, ConfigError> {
    match value {
        Value::String(text) => Ok(Value::String(interpolate_string(&text)?)),
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(interpolate_value(item)?);
            }
            Ok(Value::Array(out))
        }
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, child) in map {
                out.insert(key, interpolate_value(child)?);
            }
            Ok(Value::Object(out))
        }
        other => Ok(other),
    }
}

fn interpolate_string(input: &str) -> Result<String, ConfigError> {
    let mut remaining = input;
    let mut out = String::new();
    while let Some(start) = remaining.find("${") {
        out.push_str(&remaining[..start]);
        remaining = &remaining[start + 2..];
        let end = remaining
            .find('}')
            .ok_or(ConfigError::InvalidInterpolation)?;
        let inner = &remaining[..end];
        remaining = &remaining[end + 1..];
        let (name, default) = match inner.split_once(":-") {
            Some((name, default)) => (name, Some(default)),
            None => (inner, None),
        };
        if !is_ident(name) {
            return Err(ConfigError::InvalidInterpolation);
        }
        match std::env::var(name) {
            Ok(value) => out.push_str(&value),
            Err(_) => match default {
                Some(fallback) => out.push_str(fallback),
                None => {
                    return Err(ConfigError::MissingEnvironment {
                        name: name.to_owned(),
                    });
                }
            },
        }
    }
    out.push_str(remaining);
    Ok(out)
}

fn is_ident(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn flatten(prefix: &str, value: &Value, out: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(&next, child, out);
            }
        }
        Value::String(text) => {
            if !prefix.is_empty() {
                out.insert(prefix.to_owned(), text.clone());
            }
        }
        Value::Number(number) => {
            if !prefix.is_empty() {
                out.insert(prefix.to_owned(), number.to_string());
            }
        }
        Value::Bool(flag) => {
            if !prefix.is_empty() {
                out.insert(prefix.to_owned(), flag.to_string());
            }
        }
        Value::Null | Value::Array(_) => {}
    }
}

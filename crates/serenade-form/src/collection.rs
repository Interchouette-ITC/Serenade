//! Collection field names, prototypes, and nested data helpers.

use std::collections::{BTreeMap, HashMap};

use crate::{FormError, escape_attr};

/// Placeholder index token used in HTML collection prototypes.
pub const COLLECTION_PROTOTYPE_INDEX: &str = "__name__";

/// Builds an indexed collection field name: `items[0][name]`.
///
/// # Examples
///
/// ```
/// use serenade_form::collection_field_name;
///
/// assert_eq!(
///     collection_field_name("items", "0", "name"),
///     "items[0][name]"
/// );
/// ```
#[must_use]
pub fn collection_field_name(collection: &str, index: &str, field: &str) -> String {
    format!("{collection}[{index}][{field}]")
}

/// Builds a prototype field name using [`COLLECTION_PROTOTYPE_INDEX`].
#[must_use]
pub fn collection_prototype_field_name(collection: &str, field: &str) -> String {
    collection_field_name(collection, COLLECTION_PROTOTYPE_INDEX, field)
}

/// Metadata for progressive-enhancement HTML (no JS framework in core).
///
/// Apps can emit these as `data-*` attributes and replace `__name__` in a
/// prototype row when adding entries client-side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionPrototypeMeta {
    /// Collection base name (for example `items`).
    pub name: String,
    /// Index placeholder (`__name__`).
    pub prototype_index: &'static str,
}

impl CollectionPrototypeMeta {
    /// Creates prototype metadata for `collection`.
    #[must_use]
    pub fn new(collection: impl Into<String>) -> Self {
        Self {
            name: collection.into(),
            prototype_index: COLLECTION_PROTOTYPE_INDEX,
        }
    }

    /// Prototype field name for a child field under this collection.
    #[must_use]
    pub fn field_name(&self, field: &str) -> String {
        collection_prototype_field_name(&self.name, field)
    }

    /// Escaped `data-collection` and `data-prototype-index` attribute snippet.
    #[must_use]
    pub fn data_attributes_html(&self) -> String {
        format!(
            r#"data-collection="{}" data-prototype-index="{}""#,
            escape_attr(&self.name),
            escape_attr(self.prototype_index),
        )
    }
}

/// Nested tree node produced by [`unflatten_form_data`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestedData {
    /// Leaf string value.
    Value(String),
    /// Nested map (compound or collection index).
    Map(BTreeMap<String, Self>),
}

impl NestedData {
    /// Returns the leaf string when this node is a value.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Value(v) => Some(v.as_str()),
            Self::Map(_) => None,
        }
    }

    /// Returns the nested map when this node is a map.
    #[must_use]
    pub const fn as_map(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Map(map) => Some(map),
            Self::Value(_) => None,
        }
    }
}

/// Builds a nested map from flat urlencoded keys (`address.street`, `items[0][name]`).
///
/// # Errors
///
/// Returns [`FormError::NestedConflict`] when the same path is both a leaf and a map.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use serenade_form::{unflatten_form_data, NestedData};
///
/// let mut flat = HashMap::new();
/// flat.insert("address.street".into(), "Main".into());
/// let nested = unflatten_form_data(&flat).unwrap();
/// assert_eq!(
///     nested["address"].as_map().unwrap()["street"].as_str(),
///     Some("Main")
/// );
/// ```
pub fn unflatten_form_data<S: ::std::hash::BuildHasher>(
    flat: &HashMap<String, String, S>,
) -> Result<BTreeMap<String, NestedData>, FormError> {
    let mut root = BTreeMap::new();
    for (key, value) in flat {
        let segments = key_segments(key);
        if segments.is_empty() {
            continue;
        }
        insert_path(&mut root, &segments, value.clone())?;
    }
    Ok(root)
}

/// Lists numeric indices present for `collection` in a flat map (`items[0][…]`).
#[must_use]
pub fn collection_indices<S: ::std::hash::BuildHasher>(
    flat: &HashMap<String, String, S>,
    collection: &str,
) -> Vec<usize> {
    let prefix = format!("{collection}[");
    let mut indices = Vec::new();
    for key in flat.keys() {
        if let Some(rest) = key.strip_prefix(&prefix)
            && let Some((index_str, _)) = rest.split_once(']')
            && let Ok(index) = index_str.parse::<usize>()
            && !indices.contains(&index)
        {
            indices.push(index);
        }
    }
    indices.sort_unstable();
    indices
}

fn key_segments(key: &str) -> Vec<String> {
    let normalized = key.replace("][", ".").replace('[', ".").replace(']', "");
    normalized
        .split('.')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

fn insert_path(
    map: &mut BTreeMap<String, NestedData>,
    segments: &[String],
    value: String,
) -> Result<(), FormError> {
    let Some((first, rest)) = segments.split_first() else {
        return Ok(());
    };
    if rest.is_empty() {
        match map.get(first) {
            Some(NestedData::Map(_)) => {
                return Err(FormError::NestedConflict(first.clone()));
            }
            Some(NestedData::Value(_)) | None => {
                map.insert(first.clone(), NestedData::Value(value));
            }
        }
        return Ok(());
    }
    let entry = map
        .entry(first.clone())
        .or_insert_with(|| NestedData::Map(BTreeMap::new()));
    match entry {
        NestedData::Map(child) => insert_path(child, rest, value),
        NestedData::Value(_) => Err(FormError::NestedConflict(first.clone())),
    }
}

#[cfg(test)]
mod insert_path_tests {
    use super::*;

    #[test]
    fn empty_segments_are_noop() {
        let mut map = BTreeMap::new();
        insert_path(&mut map, &[], String::from("x")).expect("empty");
        assert!(map.is_empty());
    }

    #[test]
    fn leaf_then_child_conflicts() {
        let mut map = BTreeMap::new();
        insert_path(&mut map, &[String::from("geo")], String::from("leaf")).expect("leaf");
        let err = insert_path(
            &mut map,
            &[String::from("geo"), String::from("lat")],
            String::from("1"),
        )
        .expect_err("conflict");
        assert!(matches!(err, FormError::NestedConflict(path) if path == "geo"));
    }

    #[test]
    fn child_then_leaf_conflicts() {
        let mut map = BTreeMap::new();
        insert_path(
            &mut map,
            &[String::from("geo"), String::from("lat")],
            String::from("1"),
        )
        .expect("child");
        let err = insert_path(&mut map, &[String::from("geo")], String::from("leaf"))
            .expect_err("conflict");
        assert!(matches!(err, FormError::NestedConflict(path) if path == "geo"));
    }
}

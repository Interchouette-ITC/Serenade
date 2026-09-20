//! Current places for a subject.

use std::collections::BTreeSet;

/// Set of places a subject currently occupies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Marking {
    places: BTreeSet<String>,
}

impl Marking {
    /// Empty marking (no places).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Single-place marking (state-machine style).
    #[must_use]
    pub fn single(place: impl Into<String>) -> Self {
        let mut places = BTreeSet::new();
        places.insert(place.into());
        Self { places }
    }

    /// Marking from an iterator of place names.
    #[must_use]
    pub fn from_places<I, S>(places: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            places: places.into_iter().map(Into::into).collect(),
        }
    }

    /// Whether this marking contains `place`.
    #[must_use]
    pub fn has(&self, place: &str) -> bool {
        self.places.contains(place)
    }

    /// Whether every place in `required` is present.
    #[must_use]
    pub fn has_all<'a, I>(&self, required: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        required.into_iter().all(|p| self.has(p))
    }

    /// Place names in sorted order.
    pub fn places(&self) -> impl Iterator<Item = &str> {
        self.places.iter().map(String::as_str)
    }

    /// Number of places.
    #[must_use]
    pub fn len(&self) -> usize {
        self.places.len()
    }

    /// Whether the marking has no places.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.places.is_empty()
    }

    /// Insert a place.
    pub fn mark(&mut self, place: impl Into<String>) {
        self.places.insert(place.into());
    }

    /// Remove a place.
    pub fn unmark(&mut self, place: &str) {
        self.places.remove(place);
    }
}

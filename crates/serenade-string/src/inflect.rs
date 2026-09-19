//! English pluralize / singularize (limited irregular table).

/// Irregular singular → plural pairs (lowercase).
const IRREGULAR: &[(&str, &str)] = &[
    ("person", "people"),
    ("man", "men"),
    ("woman", "women"),
    ("child", "children"),
    ("tooth", "teeth"),
    ("foot", "feet"),
    ("mouse", "mice"),
    ("goose", "geese"),
    ("ox", "oxen"),
    ("index", "indices"),
    ("matrix", "matrices"),
    ("vertex", "vertices"),
    ("quiz", "quizzes"),
    ("bus", "buses"),
];

/// Uncountable nouns (lowercase) that do not change.
const UNCOUNTABLE: &[&str] = &[
    "sheep",
    "fish",
    "deer",
    "moose",
    "series",
    "species",
    "news",
    "information",
    "equipment",
];

/// English plural form of `word`.
///
/// Covers common suffixes (`y` → `ies`, `s/x/z/ch/sh` → `es`, `f/fe` → `ves`)
/// plus a small irregular table. Not a full linguistic Inflector and not i18n.
#[must_use]
pub fn pluralize(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }
    let lower = word.to_ascii_lowercase();
    if UNCOUNTABLE.contains(&lower.as_str()) {
        return word.to_owned();
    }
    if let Some((_, plural)) = IRREGULAR.iter().find(|(singular, _)| *singular == lower) {
        return restore_case(word, plural);
    }
    if lower.ends_with('y')
        && lower
            .chars()
            .nth(lower.len().saturating_sub(2))
            .is_some_and(|ch| !is_vowel(ch))
    {
        let stem = &lower[..lower.len() - 1];
        return restore_case(word, &format!("{stem}ies"));
    }
    if lower.ends_with("fe") {
        let stem = &lower[..lower.len() - 2];
        return restore_case(word, &format!("{stem}ves"));
    }
    if lower.ends_with('f') {
        let stem = &lower[..lower.len() - 1];
        return restore_case(word, &format!("{stem}ves"));
    }
    if lower.ends_with("ch")
        || lower.ends_with("sh")
        || lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with('o')
    {
        return restore_case(word, &format!("{lower}es"));
    }
    restore_case(word, &format!("{lower}s"))
}

/// English singular form of `word` (best-effort reverse of [`pluralize`]).
#[must_use]
pub fn singularize(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }
    let lower = word.to_ascii_lowercase();
    if UNCOUNTABLE.contains(&lower.as_str()) {
        return word.to_owned();
    }
    if let Some((singular, _)) = IRREGULAR.iter().find(|(_, plural)| *plural == lower) {
        return restore_case(word, singular);
    }
    if lower.ends_with("ies") && lower.len() > 3 {
        let stem = &lower[..lower.len() - 3];
        return restore_case(word, &format!("{stem}y"));
    }
    if lower.ends_with("ves") && lower.len() > 3 {
        let stem = &lower[..lower.len() - 3];
        return restore_case(word, &format!("{stem}f"));
    }
    if lower.ends_with("ches")
        || lower.ends_with("shes")
        || lower.ends_with("sses")
        || lower.ends_with("xes")
        || lower.ends_with("zes")
        || lower.ends_with("oes")
    {
        return restore_case(word, &lower[..lower.len() - 2]);
    }
    if lower.ends_with('s') && !lower.ends_with("ss") && lower.len() > 1 {
        return restore_case(word, &lower[..lower.len() - 1]);
    }
    word.to_owned()
}

const fn is_vowel(ch: char) -> bool {
    matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u')
}

fn restore_case(original: &str, transformed: &str) -> String {
    if original
        .chars()
        .all(|ch| !ch.is_ascii_alphabetic() || ch.is_ascii_uppercase())
        && original.chars().any(|ch| ch.is_ascii_alphabetic())
    {
        return transformed.to_ascii_uppercase();
    }
    if original
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        let mut chars = transformed.chars();
        let Some(first) = chars.next() else {
            return String::new();
        };
        let mut out = first.to_ascii_uppercase().to_string();
        out.push_str(chars.as_str());
        return out;
    }
    transformed.to_owned()
}

#[cfg(test)]
mod inflect_edge_tests {
    use super::{pluralize, restore_case, singularize};

    #[test]
    fn singularize_unchanged_when_no_rule_matches() {
        assert_eq!(singularize("cat"), "cat");
        assert_eq!(singularize("ss"), "ss");
    }

    #[test]
    fn restore_case_all_caps_and_empty_transformed() {
        assert_eq!(pluralize("BOX"), "BOXES");
        assert_eq!(pluralize("CAT"), "CATS");
        assert_eq!(singularize("BOXES"), "BOX");
        assert_eq!(restore_case("A", ""), "");
        assert_eq!(restore_case("", "x"), "x");
    }
}

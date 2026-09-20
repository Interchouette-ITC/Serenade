//! Case transforms.

/// Splits `input` into lowercase words on non-alphanumeric boundaries and
/// camel/Pascal/snake/kebab transitions.
fn words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if !ch.is_ascii_alphanumeric() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        let lower = ch.to_ascii_lowercase();
        if ch.is_ascii_uppercase() && !current.is_empty() {
            let next_lower = chars.peek().is_some_and(char::is_ascii_lowercase);
            if next_lower
                || current
                    .chars()
                    .last()
                    .is_some_and(|ch| ch.is_ascii_lowercase())
            {
                words.push(std::mem::take(&mut current));
            }
        }
        current.push(lower);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// `hello_world` style.
#[must_use]
pub fn snake_case(input: &str) -> String {
    words(input).join("_")
}

/// `hello-world` style.
#[must_use]
pub fn kebab_case(input: &str) -> String {
    words(input).join("-")
}

/// `helloWorld` style.
#[must_use]
pub fn camel_case(input: &str) -> String {
    let parts = words(input);
    let mut iter = parts.into_iter();
    let Some(first) = iter.next() else {
        return String::new();
    };
    let mut out = first;
    for part in iter {
        out.push_str(&capitalize(&part));
    }
    out
}

/// `HelloWorld` style.
#[must_use]
pub fn pascal_case(input: &str) -> String {
    words(input)
        .into_iter()
        .map(|part| capitalize(&part))
        .collect()
}

/// `Hello World` style (space-separated capitalized words).
#[must_use]
pub fn title_case(input: &str) -> String {
    words(input)
        .into_iter()
        .map(|part| capitalize(&part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = first.to_ascii_uppercase().to_string();
    out.push_str(chars.as_str());
    out
}

#[cfg(test)]
mod case_edge_tests {
    use super::{camel_case, capitalize, kebab_case, pascal_case, snake_case, title_case};

    #[test]
    fn splits_when_cap_run_follows_lowercase() {
        assert_eq!(snake_case("AbcDEF"), "abc_d_e_f");
        assert_eq!(kebab_case("fooHTTP"), "foo-h-t-t-p");
        assert_eq!(camel_case("AbcDEF"), "abcDEF");
    }

    #[test]
    fn capitalize_empty_word() {
        assert_eq!(capitalize(""), "");
        assert_eq!(pascal_case(""), "");
        assert_eq!(title_case(""), "");
    }
}

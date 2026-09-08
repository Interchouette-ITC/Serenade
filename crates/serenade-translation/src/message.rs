//! ICU `MessageFormat` subset: `{name}` placeholders and `{count, plural, …}`.

use crate::Locale;

/// Formats a catalogue template with named parameters.
///
/// Supports:
/// - Simple placeholders: `Hello {name}`
/// - ICU plural blocks: `{count, plural, one {# item} other {# items}}`
/// - Exact matches: `=0 {none}` inside a plural block
///
/// Unknown placeholders are left unchanged. `#` inside a plural arm is replaced
/// with the absolute plural number.
#[must_use]
pub fn format_message(
    template: &str,
    parameters: &[(&str, &str)],
    locale: &Locale,
    plural_number: Option<i64>,
) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = find_matching_brace(after) else {
            out.push('{');
            rest = after;
            continue;
        };
        let inner = &after[..end];
        rest = &after[end + 1..];
        out.push_str(&expand_placeholder(
            inner,
            parameters,
            locale,
            plural_number,
        ));
    }
    out.push_str(rest);
    out
}

fn param<'a>(parameters: &'a [(&str, &str)], name: &str) -> Option<&'a str> {
    parameters
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| *value)
}

fn find_matching_brace(input: &str) -> Option<usize> {
    let mut depth = 0_usize;
    for (idx, ch) in input.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' if depth == 0 => return Some(idx),
            '}' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn expand_placeholder(
    inner: &str,
    parameters: &[(&str, &str)],
    locale: &Locale,
    plural_number: Option<i64>,
) -> String {
    let trimmed = inner.trim();
    if let Some((name, rest)) = trimmed.split_once(',') {
        let name = name.trim();
        let rest = rest.trim();
        if let Some(plural_body) = rest.strip_prefix("plural") {
            let plural_body = plural_body.trim().trim_start_matches(',').trim();
            let number = plural_number
                .or_else(|| param(parameters, name).and_then(|v| v.parse().ok()))
                .unwrap_or(0);
            return expand_plural(plural_body, number, parameters, locale);
        }
    }
    param(parameters, trimmed).map_or_else(|| format!("{{{trimmed}}}"), ToOwned::to_owned)
}

fn expand_plural(body: &str, number: i64, parameters: &[(&str, &str)], locale: &Locale) -> String {
    let category = plural_category(locale, number);
    let arms = parse_plural_arms(body);
    let exact = format!("={number}");
    let chosen = arms
        .iter()
        .find(|(key, _)| key == &exact)
        .or_else(|| arms.iter().find(|(key, _)| key == &category))
        .or_else(|| arms.iter().find(|(key, _)| key == "other"))
        .map_or(body, |(_, text)| text.as_str());

    let with_hash = chosen.replace('#', &number.abs().to_string());
    format_message(&with_hash, parameters, locale, Some(number))
}

/// Parses ICU plural arms (`one {…} other {…}`).
#[must_use]
pub fn parse_plural_arms(body: &str) -> Vec<(String, String)> {
    let mut arms = Vec::new();
    let mut rest = body.trim();
    while !rest.is_empty() {
        let rest_trim = rest.trim_start();
        let (key, after_key) = match rest_trim.find('{') {
            Some(idx) => (rest_trim[..idx].trim().to_owned(), &rest_trim[idx + 1..]),
            None => break,
        };
        let Some(end) = find_matching_brace(after_key) else {
            break;
        };
        let value = after_key[..end].to_owned();
        arms.push((key, value));
        rest = &after_key[end + 1..];
    }
    arms
}

#[cfg(feature = "icu")]
fn plural_category(locale: &Locale, number: i64) -> String {
    use icu_locale::Locale as IcuLocale;
    use icu_plurals::{PluralCategory, PluralRuleType, PluralRules, PluralRulesOptions};

    let icu_locale = locale
        .as_str()
        .parse::<IcuLocale>()
        .unwrap_or_else(|_| "en".parse().expect("en"));
    let options = PluralRulesOptions::from(PluralRuleType::Cardinal);
    let rules = PluralRules::try_new(icu_locale.into(), options).unwrap_or_else(|_| {
        let en: IcuLocale = "en".parse().expect("en");
        PluralRules::try_new(
            en.into(),
            PluralRulesOptions::from(PluralRuleType::Cardinal),
        )
        .expect("en plural rules")
    });
    let category = rules.category_for(number.unsigned_abs());
    match category {
        PluralCategory::Zero => "zero",
        PluralCategory::One => "one",
        PluralCategory::Two => "two",
        PluralCategory::Few => "few",
        PluralCategory::Many => "many",
        PluralCategory::Other => "other",
    }
    .to_owned()
}

#[cfg(not(feature = "icu"))]
fn plural_category(_locale: &Locale, number: i64) -> String {
    if number == 1 || number == -1 {
        "one".into()
    } else {
        "other".into()
    }
}

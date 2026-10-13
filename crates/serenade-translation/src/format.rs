//! Locale-aware number, currency, date, and display-name helpers (ICU4X).

use crate::Locale;

/// Formats an integer in a locale-sensitive way.
///
/// With the `icu` feature this uses ICU4X decimal data; without it, falls back
/// to plain decimal digits. Locales that ICU cannot parse also fall back to
/// plain digits.
///
/// # Panics
///
/// With the `icu` feature, panics only if compiled ICU decimal data cannot
/// build a formatter for a successfully parsed locale.
#[must_use]
pub fn format_number(value: i64, locale: &Locale) -> String {
    #[cfg(feature = "icu")]
    {
        use icu_decimal::DecimalFormatter;
        use icu_decimal::input::Decimal;
        use icu_decimal::options::{DecimalFormatterOptions, GroupingStrategy};
        use icu_locale::Locale as IcuLocale;
        use writeable::Writeable;

        let Some(icu_locale) = locale.as_str().parse::<IcuLocale>().ok() else {
            return value.to_string();
        };
        let mut options = DecimalFormatterOptions::default();
        options.grouping_strategy = Some(GroupingStrategy::Auto);
        let formatter =
            DecimalFormatter::try_new(icu_locale.into(), options).expect("icu decimal data");
        formatter
            .format(&Decimal::from(value))
            .write_to_string()
            .into_owned()
    }
    #[cfg(not(feature = "icu"))]
    {
        let _ = locale;
        value.to_string()
    }
}

/// Formats a floating value with up to `fraction_digits` fraction digits.
///
/// With the `icu` feature, uses locale grouping and decimal separators from
/// ICU4X. Without it, falls back to plain ASCII digits.
#[must_use]
pub fn format_number_f64(value: f64, locale: &Locale, fraction_digits: u8) -> String {
    #[cfg(feature = "icu")]
    {
        icu_format_number(value, locale, fraction_digits)
    }
    #[cfg(not(feature = "icu"))]
    {
        let _ = locale;
        plain_number(value, fraction_digits)
    }
}

/// Formats a monetary amount for `currency` (ISO 4217 alphabetic code) and `locale`.
///
/// With the `icu` feature this uses ICU4X currency patterns (symbol placement,
/// grouping, and decimal separators). Without it, or when the locale / currency
/// code cannot be used, falls back to `{number} {CURRENCY}` with two fraction
/// digits.
///
/// # Examples
///
/// ```
/// use serenade_translation::{Locale, format_currency};
///
/// let en = Locale::new("en-US").unwrap();
/// let text = format_currency(12.5, "USD", &en);
/// assert!(!text.is_empty());
/// ```
#[must_use]
pub fn format_currency(amount: f64, currency: &str, locale: &Locale) -> String {
    #[cfg(feature = "icu")]
    {
        if let Some(formatted) = icu_format_currency(amount, currency, locale) {
            return formatted;
        }
    }
    let number = format_number_f64(amount, locale, 2);
    let code = normalize_currency_code(currency).unwrap_or_else(|| currency.to_owned());
    format!("{number} {code}")
}

/// Returns `true` when `code` is a well-formed ISO 4217 alphabetic currency code
/// (exactly three ASCII letters). Does not consult the ISO registry.
#[must_use]
pub fn is_currency_code(code: &str) -> bool {
    normalize_currency_code(code).is_some()
}

/// Normalizes an ISO 4217 alphabetic currency code to uppercase.
///
/// Returns [`None`] when `code` is not exactly three ASCII letters.
#[must_use]
pub fn normalize_currency_code(code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.len() != 3 || !trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some(trimmed.to_ascii_uppercase())
}

/// Localized display name for a BCP 47 language subtag in `display_locale`.
///
/// Requires the `icu` feature. Returns [`None`] when ICU data has no name, the
/// language tag is invalid, or `icu` is disabled.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "icu")]
/// # {
/// use serenade_translation::{Locale, language_display_name};
///
/// let en = Locale::new("en").unwrap();
/// assert_eq!(language_display_name("fr", &en).as_deref(), Some("French"));
/// # }
/// ```
#[must_use]
pub fn language_display_name(language: &str, display_locale: &Locale) -> Option<String> {
    #[cfg(feature = "icu")]
    {
        icu_language_display_name(language, display_locale)
    }
    #[cfg(not(feature = "icu"))]
    {
        let _ = (language, display_locale);
        None
    }
}

/// Localized display name for a BCP 47 region subtag in `display_locale`.
///
/// Requires the `icu` feature. Returns [`None`] when ICU data has no name, the
/// region tag is invalid, or `icu` is disabled.
#[must_use]
pub fn region_display_name(region: &str, display_locale: &Locale) -> Option<String> {
    #[cfg(feature = "icu")]
    {
        icu_region_display_name(region, display_locale)
    }
    #[cfg(not(feature = "icu"))]
    {
        let _ = (region, display_locale);
        None
    }
}

/// Formats a Gregorian calendar date (`year`, `month`, `day`) for `locale`.
///
/// Without the `icu` feature, returns ISO-8601 `YYYY-MM-DD`.
#[must_use]
pub fn format_date(year: i32, month: u8, day: u8, locale: &Locale) -> String {
    #[cfg(feature = "icu")]
    {
        icu_format_date(year, month, day, locale)
    }
    #[cfg(not(feature = "icu"))]
    {
        let _ = locale;
        format!("{year:04}-{month:02}-{day:02}")
    }
}

fn plain_number(value: f64, fraction_digits: u8) -> String {
    if fraction_digits == 0 {
        format!("{value:.0}")
    } else {
        format!("{value:.prec$}", prec = usize::from(fraction_digits))
    }
}

#[cfg(feature = "icu")]
fn decimal_from_f64(value: f64, fraction_digits: u8) -> icu_decimal::input::Decimal {
    use icu_decimal::input::Decimal;

    let scale = 10_i64.pow(u32::from(fraction_digits));
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let scaled = (value * scale as f64).round() as i64;
    let mut decimal = Decimal::from(scaled);
    if fraction_digits > 0 {
        decimal.multiply_pow10(-i16::from(fraction_digits));
    }
    decimal
}

#[cfg(feature = "icu")]
fn icu_format_number(value: f64, locale: &Locale, fraction_digits: u8) -> String {
    use icu_decimal::DecimalFormatter;
    use icu_decimal::options::{DecimalFormatterOptions, GroupingStrategy};
    use icu_locale::Locale as IcuLocale;
    use writeable::Writeable;

    let Some(icu_locale) = locale.as_str().parse::<IcuLocale>().ok() else {
        return plain_number(value, fraction_digits);
    };
    let decimal = decimal_from_f64(value, fraction_digits);
    let mut options = DecimalFormatterOptions::default();
    options.grouping_strategy = Some(GroupingStrategy::Auto);
    let formatter =
        DecimalFormatter::try_new(icu_locale.into(), options).expect("icu decimal data");
    formatter.format(&decimal).write_to_string().into_owned()
}

#[cfg(feature = "icu")]
fn icu_format_currency(amount: f64, currency: &str, locale: &Locale) -> Option<String> {
    use icu_experimental::dimension::currency::CurrencyCode;
    use icu_experimental::dimension::currency::formatter::CurrencyFormatter;
    use icu_experimental::dimension::currency::options::CurrencyFormatterOptions;
    use icu_locale::Locale as IcuLocale;
    use tinystr::TinyAsciiStr;
    use writeable::Writeable;

    let code = normalize_currency_code(currency)?;
    let tiny = TinyAsciiStr::<3>::try_from_str(&code).ok()?;
    let icu_locale = locale.as_str().parse::<IcuLocale>().ok()?;
    let decimal = decimal_from_f64(amount, 2);
    let currency_fmt =
        CurrencyFormatter::try_new(icu_locale.into(), CurrencyFormatterOptions::default()).ok()?;
    let text = currency_fmt.format_fixed_decimal(&decimal, CurrencyCode(tiny));
    Some(text.write_to_string().into_owned())
}

#[cfg(feature = "icu")]
fn icu_language_display_name(language: &str, display_locale: &Locale) -> Option<String> {
    use icu_experimental::displaynames::{DisplayNamesOptions, LanguageDisplayNames};
    use icu_locale::Locale as IcuLocale;
    use icu_locale::subtags::Language;

    let lang = Language::try_from_str(language.trim()).ok()?;
    let icu_locale = display_locale.as_str().parse::<IcuLocale>().ok()?;
    let names =
        LanguageDisplayNames::try_new(icu_locale.into(), DisplayNamesOptions::default()).ok()?;
    names.of(lang).map(ToOwned::to_owned)
}

#[cfg(feature = "icu")]
fn icu_region_display_name(region: &str, display_locale: &Locale) -> Option<String> {
    use icu_experimental::displaynames::{DisplayNamesOptions, RegionDisplayNames};
    use icu_locale::Locale as IcuLocale;
    use icu_locale::subtags::Region;

    let region = Region::try_from_str(region.trim()).ok()?;
    let icu_locale = display_locale.as_str().parse::<IcuLocale>().ok()?;
    let names =
        RegionDisplayNames::try_new(icu_locale.into(), DisplayNamesOptions::default()).ok()?;
    names.of(region).map(ToOwned::to_owned)
}

#[cfg(feature = "icu")]
fn icu_format_date(year: i32, month: u8, day: u8, locale: &Locale) -> String {
    use icu_calendar::Date;
    use icu_datetime::DateTimeFormatter;
    use icu_datetime::fieldsets;
    use icu_locale::Locale as IcuLocale;
    use writeable::Writeable;

    let Some(icu_locale) = locale.as_str().parse::<IcuLocale>().ok() else {
        return format!("{year:04}-{month:02}-{day:02}");
    };
    let Ok(date) = Date::try_new_iso(year, month, day) else {
        return format!("{year:04}-{month:02}-{day:02}");
    };
    let formatter = DateTimeFormatter::try_new(icu_locale.into(), fieldsets::YMD::medium())
        .expect("icu datetime data");
    formatter.format(&date).write_to_string().into_owned()
}

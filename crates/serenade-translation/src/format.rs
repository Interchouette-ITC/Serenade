//! Locale-aware number and date formatting helpers (ICU4X).

use crate::Locale;

/// Formats an integer in a locale-sensitive way.
///
/// With the `icu` feature this uses ICU4X decimal data; without it, falls back
/// to plain decimal digits.
#[must_use]
pub fn format_number(value: i64, locale: &Locale) -> String {
    #[cfg(feature = "icu")]
    {
        use icu_decimal::DecimalFormatter;
        use icu_decimal::input::Decimal;
        use icu_decimal::options::DecimalFormatterOptions;
        use icu_locale::Locale as IcuLocale;
        use writeable::Writeable;

        let Ok(icu_locale) = locale.as_str().parse::<IcuLocale>() else {
            return value.to_string();
        };
        let Ok(formatter) =
            DecimalFormatter::try_new(icu_locale.into(), DecimalFormatterOptions::default())
        else {
            return value.to_string();
        };
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

/// Formats a monetary amount as `{number} {currency}` (ISO 4217 code).
#[must_use]
pub fn format_currency(amount: f64, currency: &str, locale: &Locale) -> String {
    let number = format_number_f64(amount, locale, 2);
    format!("{number} {currency}")
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
fn icu_format_number(value: f64, locale: &Locale, fraction_digits: u8) -> String {
    use icu_decimal::DecimalFormatter;
    use icu_decimal::input::Decimal;
    use icu_decimal::options::DecimalFormatterOptions;
    use icu_locale::Locale as IcuLocale;
    use writeable::Writeable;

    let Ok(icu_locale) = locale.as_str().parse::<IcuLocale>() else {
        return plain_number(value, fraction_digits);
    };
    let Ok(formatter) =
        DecimalFormatter::try_new(icu_locale.into(), DecimalFormatterOptions::default())
    else {
        return plain_number(value, fraction_digits);
    };

    // Scale to integer then shift decimal point (ICU Decimal API).
    let scale = 10_i64.pow(u32::from(fraction_digits));
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let scaled = (value * scale as f64).round() as i64;
    let mut decimal = Decimal::from(scaled);
    if fraction_digits > 0 {
        decimal.multiply_pow10(-i16::from(fraction_digits));
    }
    formatter.format(&decimal).write_to_string().into_owned()
}

#[cfg(feature = "icu")]
fn icu_format_date(year: i32, month: u8, day: u8, locale: &Locale) -> String {
    use icu_calendar::Date;
    use icu_datetime::DateTimeFormatter;
    use icu_datetime::fieldsets;
    use icu_locale::Locale as IcuLocale;
    use writeable::Writeable;

    let Ok(icu_locale) = locale.as_str().parse::<IcuLocale>() else {
        return format!("{year:04}-{month:02}-{day:02}");
    };
    let Ok(date) = Date::try_new_iso(year, month, day) else {
        return format!("{year:04}-{month:02}-{day:02}");
    };
    let Ok(formatter) = DateTimeFormatter::try_new(icu_locale.into(), fieldsets::YMD::medium())
    else {
        return format!("{year:04}-{month:02}-{day:02}");
    };
    formatter.format(&date).write_to_string().into_owned()
}

//! Unit tests for `serenade-translation`.

use std::sync::Arc;

use serenade_http::{HttpKernel, Method, Request, Response};
use tempfile::tempdir;

use crate::{
    CatalogueFileName, DEFAULT_DOMAIN, Locale, LocaleMiddleware, LocaleNegotiator,
    MessageCatalogue, TomlCatalogueLoader, Translator, TranslatorInterface, format_currency,
    format_date, format_message, format_number, load_directory, request_locale,
};

#[test]
fn locale_normalizes_region() {
    let locale = Locale::new("fr-fr").expect("locale");
    assert_eq!(locale.as_str(), "fr-FR");
    assert_eq!(locale.language(), "fr");
    assert_eq!(locale.region(), Some("FR"));
    assert_eq!(locale.parent_chain()[0].as_str(), "fr");
}

#[test]
fn catalogue_file_name_parses_underscore_locale() {
    let meta = CatalogueFileName::parse("messages+intl-icu.fr_FR.toml").expect("name");
    assert_eq!(meta.domain, "messages+intl-icu");
    assert_eq!(meta.locale.as_str(), "fr-FR");
    assert_eq!(meta.format, "toml");
}

#[test]
fn translator_fallback_and_placeholders() {
    let mut en = MessageCatalogue::new(Locale::new("en").unwrap());
    en.set(DEFAULT_DOMAIN, "hello", "Hello {name}");
    en.set(DEFAULT_DOMAIN, "only_en", "English only");
    let mut fr = MessageCatalogue::new(Locale::new("fr").unwrap());
    fr.set(DEFAULT_DOMAIN, "hello", "Bonjour {name}");

    let mut translator = Translator::new(Locale::new("fr").unwrap())
        .with_fallbacks(vec![Locale::new("en").unwrap()]);
    translator.add_catalogue(en);
    translator.add_catalogue(fr);

    assert_eq!(
        translator.trans("hello", &[("name", "Ada")], None, None),
        "Bonjour Ada"
    );
    assert_eq!(translator.trans("only_en", &[], None, None), "English only");
    assert_eq!(translator.trans("missing", &[], None, None), "missing");
}

#[test]
fn plural_message_selects_one_and_other() {
    let template = "{count, plural, one {# item} other {# items}}";
    let locale = Locale::new("en").unwrap();
    assert_eq!(
        format_message(template, &[("count", "1")], &locale, Some(1)),
        "1 item"
    );
    assert_eq!(
        format_message(template, &[("count", "3")], &locale, Some(3)),
        "3 items"
    );
}

#[test]
fn load_toml_directory() {
    let dir = tempdir().expect("temp");
    std::fs::write(dir.path().join("messages.en.toml"), r#"greeting = "Hi""#).expect("write");
    std::fs::write(dir.path().join("messages.fr.toml"), r#"greeting = "Salut""#).expect("write");
    let toml = TomlCatalogueLoader;
    let catalogues = load_directory(dir.path(), &[&toml]).expect("load");
    assert_eq!(catalogues.len(), 2);
    assert_eq!(
        catalogues
            .get(&Locale::new("fr").unwrap())
            .and_then(|c| c.get("greeting", DEFAULT_DOMAIN)),
        Some("Salut")
    );
}

#[test]
fn negotiator_prefers_query_then_cookie_then_accept() {
    let negotiator = LocaleNegotiator::new(Locale::new("en").unwrap())
        .with_allowed(vec![Locale::new("en").unwrap(), Locale::new("fr").unwrap()]);

    let mut request = Request::new(Method::Get, "/")
        .with_query("_locale=fr")
        .with_header("accept-language", "en")
        .with_header("cookie", "_locale=en");
    assert_eq!(negotiator.resolve(&request).as_str(), "fr");

    request = Request::new(Method::Get, "/")
        .with_header("cookie", "_locale=fr")
        .with_header("accept-language", "en");
    assert_eq!(negotiator.resolve(&request).as_str(), "fr");

    request = Request::new(Method::Get, "/").with_header("accept-language", "fr-FR,fr;q=0.9");
    assert_eq!(negotiator.resolve(&request).as_str(), "fr");
}

#[test]
fn locale_middleware_sets_attribute() {
    let negotiator = LocaleNegotiator::new(Locale::new("en").unwrap())
        .with_allowed(vec![Locale::new("en").unwrap(), Locale::new("fr").unwrap()]);
    let mut kernel = HttpKernel::new(|request: &mut Request| {
        let locale = request_locale(request).expect("locale");
        Ok(Response::text(200, locale.as_str()))
    });
    kernel.push_middleware(LocaleMiddleware::new(negotiator));
    let response = kernel.handle(
        Request::new(Method::Get, "/")
            .with_query("_locale=fr")
            .with_header("accept-language", "en"),
    );
    assert_eq!(response.body_str(), Some("fr"));
}

#[test]
fn format_helpers_smoke() {
    let en = Locale::new("en").unwrap();
    let fr = Locale::new("fr").unwrap();
    assert_ne!(format_number(1_234, &en), "");
    assert_ne!(format_currency(12.5, "EUR", &fr), "");
    assert_ne!(format_date(2026, 9, 8, &fr), "");
}

#[test]
fn arc_translator_interface() {
    let mut catalogue = MessageCatalogue::new(Locale::new("en").unwrap());
    catalogue.set(DEFAULT_DOMAIN, "ok", "OK");
    let mut translator = Translator::new(Locale::new("en").unwrap());
    translator.add_catalogue(catalogue);
    let shared = Arc::new(translator);
    assert_eq!(shared.trans("ok", &[], None, None), "OK");
}

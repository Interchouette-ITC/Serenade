//! Unit tests for `serenade-translation`.

use std::path::Path;
use std::sync::Arc;

use serenade_bundle::Extension;
use serenade_config::Config;
use serenade_di::ContainerBuilder;
use serenade_http::{HttpKernel, Method, Request, Response};
use serenade_kernel::BundleInterface;
use tempfile::tempdir;

use crate::{
    CatalogueFileName, CatalogueLoader, DEFAULT_DOMAIN, DEFAULT_LOCALE_COOKIE,
    DEFAULT_LOCALE_QUERY, INTL_DOMAIN_SUFFIX, JsonCatalogueLoader, Locale, LocaleMiddleware,
    LocaleNegotiator, MessageCatalogue, TRANSLATION_BUNDLE, TRANSLATOR_SERVICE,
    TomlCatalogueLoader, TranslationBundle, TranslationExtension, Translator, TranslatorInterface,
    format_currency, format_date, format_message, format_number, format_number_f64, load_directory,
    load_paths, request_locale, version,
};

#[test]
fn crate_version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn locale_normalizes_and_rejects_invalid() {
    let locale = Locale::new("fr-fr").expect("locale");
    assert_eq!(locale.as_str(), "fr-FR");
    assert_eq!(locale.language(), "fr");
    assert_eq!(locale.region(), Some("FR"));
    assert_eq!(locale.parent_chain()[0].as_str(), "fr");
    assert_eq!(locale.as_ref(), "fr-FR");
    assert_eq!(locale.to_string(), "fr-FR");
    assert_eq!("en".parse::<Locale>().unwrap().as_str(), "en");
    assert!(Locale::new("").is_err());
    assert!(Locale::new("12").is_err());
    assert!(Locale::new("en-!!").is_err());
    assert_eq!(Locale::new("en").unwrap().parent_chain().len(), 0);
    assert!(Locale::new("en-").unwrap().region().is_none());
}

#[test]
fn catalogue_domains_intl_merge_and_has() {
    let mut bag = MessageCatalogue::new(Locale::new("en").unwrap());
    bag.set(DEFAULT_DOMAIN, "plain", "A");
    bag.set(format!("{DEFAULT_DOMAIN}{INTL_DOMAIN_SUFFIX}"), "icu", "B");
    bag.set(DEFAULT_DOMAIN, "both", "plain-wins");
    bag.set(
        format!("{DEFAULT_DOMAIN}{INTL_DOMAIN_SUFFIX}"),
        "both",
        "icu-loses",
    );
    bag.set("admin", "title", "Admin");
    assert!(bag.has("plain", DEFAULT_DOMAIN));
    assert!(bag.has("icu", DEFAULT_DOMAIN));
    assert_eq!(bag.get("icu", DEFAULT_DOMAIN), Some("B"));
    assert_eq!(
        bag.domains(),
        vec!["admin".to_owned(), "messages".to_owned()]
    );
    let all = bag.all(DEFAULT_DOMAIN);
    assert_eq!(all.get("plain").map(String::as_str), Some("A"));
    assert_eq!(all.get("icu").map(String::as_str), Some("B"));
    assert_eq!(all.get("both").map(String::as_str), Some("plain-wins"));

    let mut other = MessageCatalogue::new(Locale::new("en").unwrap());
    other.set(DEFAULT_DOMAIN, "plain", "Z");
    other.set(DEFAULT_DOMAIN, "extra", "X");
    bag.add_catalogue(&other);
    assert_eq!(bag.get("plain", DEFAULT_DOMAIN), Some("A"));
    assert_eq!(bag.get("extra", DEFAULT_DOMAIN), Some("X"));
    bag.replace_catalogue(&other);
    assert_eq!(bag.get("plain", DEFAULT_DOMAIN), Some("Z"));
}

#[test]
fn catalogue_file_name_parses_and_rejects() {
    let meta = CatalogueFileName::parse("messages+intl-icu.fr_FR.toml").expect("name");
    assert_eq!(meta.domain, "messages+intl-icu");
    assert_eq!(meta.locale.as_str(), "fr-FR");
    assert_eq!(meta.format, "toml");
    assert!(CatalogueFileName::parse("nope").is_none());
    assert!(CatalogueFileName::parse("messages.toml").is_none());
}

#[test]
fn toml_and_json_loaders_cover_shapes_and_errors() {
    let path = Path::new("messages.en.toml");
    let toml = TomlCatalogueLoader;
    assert_eq!(toml.format(), "toml");
    let flat = toml.parse(br#"hi = "there""#, path).expect("flat toml");
    assert_eq!(flat.get("hi").map(String::as_str), Some("there"));
    let nested = toml
        .parse(
            br#"
[messages]
n = 1
f = 1.5
b = true
s = "ok"
"#,
            path,
        )
        .expect("nested");
    assert_eq!(nested.get("n").map(String::as_str), Some("1"));
    assert_eq!(nested.get("b").map(String::as_str), Some("true"));
    assert!(toml.parse(b"\xff", path).is_err());
    assert!(toml.parse(b"[[[", path).is_err());
    assert!(toml.parse(b"root = []", path).is_err());
    assert!(toml.parse(br#""not-a-table""#, path).is_err());
    assert!(toml.parse(br#"bad = ["x"]"#, path).is_err());

    let json = JsonCatalogueLoader;
    assert_eq!(json.format(), "json");
    let flat_json = json
        .parse(br#"{"hi":"there","n":2,"b":false}"#, path)
        .expect("flat json");
    assert_eq!(flat_json.get("hi").map(String::as_str), Some("there"));
    let nested_json = json
        .parse(br#"{"messages":{"a":"b"}}"#, path)
        .expect("nested json");
    assert_eq!(nested_json.get("a").map(String::as_str), Some("b"));
    assert!(json.parse(b"[1]", path).is_err());
    assert!(json.parse(br#"{"bad":[]}"#, path).is_err());
    assert!(json.parse(b"{", path).is_err());
}

#[test]
fn load_directory_and_paths_merge() {
    let first = tempdir().expect("temp");
    let second = tempdir().expect("temp");
    std::fs::create_dir(first.path().join("subdir")).expect("subdir");
    std::fs::write(first.path().join("readme.txt"), "skip").expect("skip");
    std::fs::write(first.path().join("messages.en.yaml"), "a: 1").expect("yaml");
    std::fs::write(first.path().join("messages.en.toml"), r#"a = "one""#).expect("write");
    std::fs::write(first.path().join("messages.en.json"), r#"{"b":"json"}"#).expect("json");
    std::fs::write(second.path().join("messages.en.toml"), r#"a = "two""#).expect("override");

    #[cfg(unix)]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let weird = first
            .path()
            .join(OsStr::from_bytes(b"messages.\xff.en.toml"));
        std::fs::write(&weird, r#"z = "nope""#).expect("non-utf8 name");
    }

    let toml = TomlCatalogueLoader;
    let json = JsonCatalogueLoader;
    let loaders: [&dyn CatalogueLoader; 2] = [&toml, &json];
    let one = load_directory(first.path(), &loaders).expect("load");
    let en = one.get(&Locale::new("en").unwrap()).expect("en");
    assert_eq!(en.get("a", DEFAULT_DOMAIN), Some("one"));
    assert_eq!(en.get("b", DEFAULT_DOMAIN), Some("json"));

    let merged = load_paths(
        &[first.path().to_path_buf(), second.path().to_path_buf()],
        &loaders,
    )
    .expect("paths");
    assert_eq!(
        merged
            .get(&Locale::new("en").unwrap())
            .and_then(|c| c.get("a", DEFAULT_DOMAIN)),
        Some("two")
    );
    assert!(load_directory(first.path().join("missing"), &loaders).is_err());
}

#[cfg(unix)]
#[test]
fn load_directory_errors_on_unreadable_catalogue() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("temp");
    let path = dir.path().join("messages.en.toml");
    std::fs::write(&path, r#"a = "one""#).expect("write");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    let toml = TomlCatalogueLoader;
    let err = load_directory(dir.path(), &[&toml]).expect_err("unreadable");
    assert!(err.to_string().contains("messages.en.toml") || err.to_string().contains("Permission"));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("restore");
}

#[test]
fn translator_load_paths_choice_and_fallback_chain() {
    let dir = tempdir().expect("temp");
    std::fs::write(
        dir.path().join("messages.en.toml"),
        r#"
hello = "Hello {name}"
items = "{count, plural, =0 {none} one {# item} other {# items}}"
only_en = "EN"
"#,
    )
    .expect("en");
    std::fs::write(
        dir.path().join("messages.fr_FR.toml"),
        r#"hello = "Bonjour {name}""#,
    )
    .expect("fr");
    std::fs::write(
        dir.path().join("messages.fr_CA.toml"),
        r#"hello = "Allô {name}""#,
    )
    .expect("fr-CA");

    let mut translator = Translator::new(Locale::new("fr-FR").unwrap()).with_fallbacks(vec![
        Locale::new("fr-CA").unwrap(),
        Locale::new("en").unwrap(),
    ]);
    translator.load_path(dir.path()).expect("load");
    assert_eq!(translator.catalogue_count(), 3);
    translator.set_locale(Locale::new("fr-FR").unwrap());
    assert_eq!(translator.locale().as_str(), "fr-FR");
    assert_eq!(
        translator.trans("hello", &[("name", "Ada")], None, None),
        "Bonjour Ada"
    );
    assert_eq!(translator.trans("only_en", &[], None, None), "EN");
    assert_eq!(translator.trans("missing", &[], None, None), "missing");
    assert_eq!(translator.trans_choice("items", 0, &[], None, None), "none");
    assert_eq!(
        translator.trans_choice("items", 1, &[], None, None),
        "1 item"
    );
    assert_eq!(
        translator.trans_choice("items", 5, &[("count", "5")], None, None),
        "5 items"
    );
    assert_eq!(
        translator.trans_choice("missing", 2, &[], None, None),
        "missing"
    );

    let other = tempdir().expect("other");
    std::fs::write(other.path().join("messages.en.toml"), r#"only_en = "NEW""#).expect("write");
    translator
        .load_paths(&[dir.path().to_path_buf(), other.path().to_path_buf()])
        .expect("paths");
    assert_eq!(
        translator.trans("only_en", &[], None, Some(&Locale::new("en").unwrap())),
        "NEW"
    );

    let mut same = MessageCatalogue::new(Locale::new("en").unwrap());
    same.set(DEFAULT_DOMAIN, "extra", "X");
    translator.add_catalogue(same);
    assert_eq!(
        translator.trans("extra", &[], None, Some(&Locale::new("en").unwrap())),
        "X"
    );
}

#[test]
fn arc_translator_interface_covers_choice() {
    let mut catalogue = MessageCatalogue::new(Locale::new("en").unwrap());
    catalogue.set(DEFAULT_DOMAIN, "ok", "OK");
    catalogue.set(
        DEFAULT_DOMAIN,
        "n",
        "{count, plural, one {one} other {many}}",
    );
    let mut translator = Translator::new(Locale::new("en").unwrap());
    translator.add_catalogue(catalogue);
    let shared = Arc::new(translator);
    assert_eq!(shared.locale().as_str(), "en");
    assert_eq!(shared.trans("ok", &[], None, None), "OK");
    assert_eq!(shared.trans_choice("n", 2, &[], None, None), "many");
}

#[test]
fn plural_message_edges() {
    let locale = Locale::new("en").unwrap();
    assert_eq!(
        format_message("Hello {name}", &[("name", "Ada")], &locale, None),
        "Hello Ada"
    );
    assert_eq!(
        format_message("Hi {missing}", &[], &locale, None),
        "Hi {missing}"
    );
    assert_eq!(
        format_message("broken {name", &[("name", "x")], &locale, None),
        "broken {name"
    );
    assert_eq!(
        format_message("{count, plural, one {#} other {#}}", &[], &locale, Some(3)),
        "3"
    );
    assert_eq!(
        format_message(
            "{count, select, other {x}}",
            &[("count", "1")],
            &locale,
            None
        ),
        "{count, select, other {x}}"
    );
    assert_eq!(
        format_message("{count, plural, one {a}", &[], &locale, Some(1)),
        "{count, plural, one {a}"
    );
    assert_eq!(
        format_message("{count, plural, one {a}    }", &[], &locale, Some(1)),
        "a"
    );
    // Arm key without `{` → empty arms → original body kept.
    assert_eq!(
        format_message("{count, plural, orphan}", &[], &locale, Some(1)),
        "orphan"
    );
    let partial = crate::message::parse_plural_arms("one {a} {unclosed");
    assert_eq!(partial.len(), 1);
    assert_eq!(partial[0].0, "one");
    assert_eq!(partial[0].1, "a");
    assert_eq!(crate::message::parse_plural_arms("one {a}   ").len(), 1);

    assert!(crate::loader::flatten_toml(&toml::Value::Boolean(true), Path::new("x.toml")).is_err());

    let ar = Locale::new("ar").unwrap();
    let ar_template = "{count, plural, zero {z} one {o} two {t} few {f} many {m} other {x}}";
    assert_eq!(format_message(ar_template, &[], &ar, Some(0)), "z");
    assert_eq!(format_message(ar_template, &[], &ar, Some(1)), "o");
    assert_eq!(format_message(ar_template, &[], &ar, Some(2)), "t");
    assert_eq!(format_message(ar_template, &[], &ar, Some(3)), "f");
    assert_eq!(format_message(ar_template, &[], &ar, Some(11)), "m");
    assert_eq!(format_message(ar_template, &[], &ar, Some(100)), "x");

    let bad_icu = Locale::new("aaaaaaaaaa").unwrap();
    assert_eq!(
        format_message("{count, plural, one {o} other {x}}", &[], &bad_icu, Some(1)),
        "o"
    );
}

#[test]
fn negotiator_covers_priority_edges() {
    let negotiator = LocaleNegotiator::new(Locale::new("en").unwrap())
        .with_allowed(vec![Locale::new("fr").unwrap()])
        .with_cookie_name("lang")
        .with_query_param("lang");
    assert_eq!(negotiator.default_locale().as_str(), "en");
    assert!(negotiator.allowed().iter().any(|l| l.as_str() == "en"));
    assert_eq!(DEFAULT_LOCALE_COOKIE, "_locale");
    assert_eq!(DEFAULT_LOCALE_QUERY, "_locale");

    let mut request = Request::new(Method::Get, "/");
    request
        .attributes_mut()
        .insert(crate::LOCALE_ATTRIBUTE, Locale::new("fr").unwrap());
    assert_eq!(negotiator.resolve(&request).as_str(), "fr");

    // Existing attribute not allowed → fall through to Accept-Language.
    request = Request::new(Method::Get, "/");
    request
        .attributes_mut()
        .insert(crate::LOCALE_ATTRIBUTE, Locale::new("de").unwrap());
    request = request.with_header("accept-language", "fr;q=0.9");
    assert_eq!(negotiator.resolve(&request).as_str(), "fr");

    // Language match on attribute (fr-FR vs allowed fr).
    let by_lang = LocaleNegotiator::new(Locale::new("en").unwrap())
        .with_allowed(vec![Locale::new("fr").unwrap()]);
    request = Request::new(Method::Get, "/");
    request
        .attributes_mut()
        .insert(crate::LOCALE_ATTRIBUTE, Locale::new("fr-FR").unwrap());
    assert_eq!(by_lang.resolve(&request).as_str(), "fr-FR");

    request = Request::new(Method::Get, "/")
        .with_query("lang=fr%2DFR&x=1")
        .with_header("cookie", "lang=en; Path=/")
        .with_header("accept-language", "en");
    assert_eq!(negotiator.resolve(&request).as_str(), "fr");

    request = Request::new(Method::Get, "/")
        .with_query("lang=%zz")
        .with_header("cookie", "other=1; bare; lang=fr")
        .with_header("accept-language", "*, de;q=0.1, fr;q=0.8");
    assert_eq!(negotiator.resolve(&request).as_str(), "fr");

    request =
        Request::new(Method::Get, "/").with_header("accept-language", ",, fr-FR;q=abc, de;q=0.1");
    assert_eq!(negotiator.resolve(&request).as_str(), "fr");

    request = Request::new(Method::Get, "/").with_header("accept-language", "de-DE");
    assert_eq!(negotiator.resolve(&request).as_str(), "en");

    request = Request::new(Method::Get, "/").with_header("cookie", "lang=nope");
    assert_eq!(negotiator.resolve(&request).as_str(), "en");

    // Invalid cookie locale tag → skip, then no match → None → default.
    request = Request::new(Method::Get, "/").with_header("cookie", "lang=12");
    assert_eq!(negotiator.resolve(&request).as_str(), "en");

    // Cookie present but name does not match.
    request = Request::new(Method::Get, "/").with_header("cookie", "other=fr");
    assert_eq!(negotiator.resolve(&request).as_str(), "en");

    // Accept-Language wildcard only.
    request = Request::new(Method::Get, "/").with_header("accept-language", "*");
    assert_eq!(negotiator.resolve(&request).as_str(), "en");

    request = Request::new(Method::Get, "/").with_query("lang=fr+FR");
    assert_eq!(negotiator.resolve(&request).as_str(), "en");

    let with_default = LocaleNegotiator::new(Locale::new("en").unwrap())
        .with_allowed(vec![Locale::new("en").unwrap(), Locale::new("fr").unwrap()]);
    request = Request::new(Method::Get, "/").with_query("_locale=fr");
    assert_eq!(with_default.resolve(&request).as_str(), "fr");
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
    let bad = Locale::new("aaaaaaaaaa").unwrap();
    assert_ne!(format_number(1_234, &en), "");
    assert_eq!(format_number(42, &bad), "42");
    assert_ne!(format_number_f64(12.5, &fr, 2), "");
    assert_ne!(format_number_f64(3.0, &en, 0), "");
    assert_eq!(format_number_f64(1.5, &bad, 1), "1.5");
    assert_eq!(format_number_f64(2.0, &bad, 0), "2");
    assert_ne!(format_currency(12.5, "EUR", &fr), "");
    assert_ne!(format_date(2026, 9, 8, &fr), "");
    assert_eq!(format_date(2026, 2, 30, &en), "2026-02-30");
    assert_eq!(format_date(2026, 9, 8, &bad), "2026-09-08");
}

#[test]
fn translation_extension_registers_translator() {
    let dir = tempdir().expect("temp");
    std::fs::write(dir.path().join("messages.en.toml"), r#"hi = "Hi""#).expect("write");
    let config = Config::from_toml(&format!(
        r#"
default_locale = "en"
fallbacks = ["en"]
paths = ["{path}"]
"#,
        path = dir.path().display()
    ))
    .expect("config");
    let mut builder = ContainerBuilder::new();
    let extension = TranslationExtension::new().with_paths(vec![dir.path().to_path_buf()]);
    assert_eq!(extension.alias(), TRANSLATION_BUNDLE);
    extension.load(&config, &mut builder).expect("load");
    let container = builder.compile().expect("compile");
    let translator = container
        .get_as::<Arc<Translator>>(TRANSLATOR_SERVICE)
        .expect("translator");
    assert_eq!(translator.trans("hi", &[], None, None), "Hi");

    let csv = Config::from_toml(
        r#"
default_locale = "en"
fallbacks = "en, fr"
paths = ""
"#,
    )
    .expect("csv");
    let mut builder = ContainerBuilder::new();
    TranslationExtension::new()
        .load(&csv, &mut builder)
        .expect("csv load");

    let ignored = Config::from_toml(
        r#"
default_locale = "en"
fallbacks = 1
paths = true
"#,
    )
    .expect("ignored list types");
    let mut builder = ContainerBuilder::new();
    TranslationExtension::new()
        .load(&ignored, &mut builder)
        .expect("ignored load");

    // Missing default_locale → "en"; invalid fallback tags skipped; bare path ignored.
    let sparse = Config::from_toml(
        r#"
fallbacks = ["!!", "fr"]
paths = ["/tmp/serenade-translation-not-a-dir-xyz"]
"#,
    )
    .expect("sparse");
    let mut builder = ContainerBuilder::new();
    TranslationExtension::new()
        .load(&sparse, &mut builder)
        .expect("sparse load");

    let with_non_string = Config::from_toml(
        r#"
default_locale = "en"
fallbacks = ["en", 1, "fr"]
paths = []
"#,
    )
    .expect("mixed array");
    let mut builder = ContainerBuilder::new();
    TranslationExtension::new()
        .load(&with_non_string, &mut builder)
        .expect("mixed load");

    let bundle = TranslationBundle;
    assert_eq!(bundle.name(), TRANSLATION_BUNDLE);
    bundle.build().expect("build");
}

#[test]
fn translation_extension_rejects_bad_default_locale() {
    let config = Config::from_toml(r#"default_locale = """#).expect("config");
    let mut builder = ContainerBuilder::new();
    let err = TranslationExtension::new()
        .load(&config, &mut builder)
        .expect_err("bad locale");
    assert!(err.to_string().contains("translation"));
}

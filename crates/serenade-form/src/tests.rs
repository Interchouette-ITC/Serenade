use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serenade_http::{Method, Request};
use serenade_security::HmacCsrfTokenManager;
use serenade_validator::NotBlank;
use serenade_validator::{ConstraintViolationList, Violation};

use super::{
    BoolToStringTransformer, COLLECTION_PROTOTYPE_INDEX, CheckboxOptions, ChoiceEntry,
    ChoiceOptions, ChoiceToValueTransformer, CollectionPrototypeMeta, DEFAULT_MAX_FILE_SIZE,
    DataTransformer, FieldConfig, FieldKind, FileOptions, FileStorage, Form, FormError, FormStatus,
    HiddenOptions, I64ToStringTransformer, NestedData, NumberOptions, PasswordOptions, TextOptions,
    TextareaOptions, collection_field_name, collection_indices, collection_prototype_field_name,
    escape_attr, escape_html, form_errors, form_label, form_row, form_widget, join_multi,
    parse_multipart, parse_urlencoded, parse_urlencoded_multi, split_multi, unflatten_form_data,
    version,
};

#[test]
fn version_is_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn escape_html_xss_payload() {
    let raw = r#"<script>alert("x")</script>&'"#;
    let escaped = escape_html(raw);
    assert!(!escaped.contains('<'));
    assert!(escaped.contains("&lt;script&gt;"));
    assert!(escaped.contains("&amp;"));
    assert!(escaped.contains("&quot;"));
    assert!(escaped.contains("&#39;"));
    assert_eq!(escape_attr(r#""quoted""#), "&quot;quoted&quot;");
}

#[test]
fn parse_urlencoded_basic() {
    let map = parse_urlencoded(b"a=hello+world&b=%3Ctag%3E").expect("parse");
    assert_eq!(map.get("a").map(String::as_str), Some("hello world"));
    assert_eq!(map.get("b").map(String::as_str), Some("<tag>"));
}

#[test]
fn parse_urlencoded_edge_cases() {
    assert!(parse_urlencoded(b"").expect("empty").is_empty());
    let map = parse_urlencoded(b"solo&x=1&=&%41=%42&&z=9").expect("pairs");
    assert_eq!(map.get("solo").map(String::as_str), Some(""));
    assert_eq!(map.get("x").map(String::as_str), Some("1"));
    assert_eq!(map.get("A").map(String::as_str), Some("B"));
    assert_eq!(map.get("z").map(String::as_str), Some("9"));
    assert!(matches!(
        parse_urlencoded(b"\xff"),
        Err(FormError::InvalidEncoding)
    ));
    assert!(matches!(
        parse_urlencoded(b"a=%zz"),
        Err(FormError::InvalidEncoding)
    ));
    assert!(matches!(
        parse_urlencoded(b"a=%a"),
        Err(FormError::InvalidEncoding)
    ));
}

#[test]
fn parse_urlencoded_multi_keeps_order() {
    let map = parse_urlencoded_multi(b"tag=a&tag=b&tag=c").expect("multi");
    assert_eq!(
        map.get("tag").map(Vec::as_slice),
        Some([String::from("a"), String::from("b"), String::from("c")].as_slice())
    );
    let last_wins = parse_urlencoded(b"tag=a&tag=b").expect("last");
    assert_eq!(last_wins.get("tag").map(String::as_str), Some("b"));
}

#[test]
fn form_csrf_and_validate_roundtrip() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment")
        .method(Method::Post)
        .action("/comment")
        .field(
            "body",
            vec![Arc::new(NotBlank) as Arc<dyn serenade_validator::Constraint>],
        )
        .build();
    assert_eq!(form.name(), "comment");
    assert!(!form.is_submitted());
    form.prepare_csrf(&mgr).expect("prepare");
    let rendered = form.render().expect("render");
    let html = rendered.as_html();
    assert!(html.contains(r#"name="_token""#));
    assert!(html.contains("method=\"POST\""));
    assert!(html.contains(r#"action="/comment""#));
    assert_eq!(rendered.as_ref(), html);

    let token_value = html
        .split("value=\"")
        .nth(1)
        .expect("value")
        .split('"')
        .next()
        .expect("end")
        .to_owned();

    let body = format!("_token={token_value}&body=hello");
    let request = Request::new(Method::Post, "/").with_body(body.into_bytes());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert!(form.is_submitted());
    assert!(form.is_valid());
    assert!(form.violations().is_empty());
    assert_eq!(form.get("body"), Some("hello"));
    assert!(form.set("body", "updated"));
    assert_eq!(form.get("body"), Some("updated"));
    assert!(!form.set("missing", "x"));
    assert_eq!(form.get("missing"), None);
    assert_eq!(form.data().get("body").map(String::as_str), Some("updated"));
    assert_eq!(form.fields()[0].kind(), FieldKind::Text);
}

#[test]
fn form_validation_fails_blank() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment")
        .csrf(false)
        .field(
            "body",
            vec![Arc::new(NotBlank) as Arc<dyn serenade_validator::Constraint>],
        )
        .build();
    form.prepare_csrf(&mgr).expect("noop csrf");
    let request = Request::new(Method::Post, "/").with_body(b"body=".to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert!(!form.is_valid());
    assert!(!form.violations().is_empty());
}

#[test]
fn form_clears_missing_fields_and_put() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment")
        .csrf(false)
        .field("body", vec![])
        .build();
    let request = Request::new(Method::Put, "/").with_body(b"other=1".to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("body"), Some(""));
}

#[test]
fn form_render_requires_csrf_token() {
    let form = Form::builder("comment").field("body", vec![]).build();
    let err = form.render().expect_err("no token");
    assert!(matches!(err, FormError::Csrf(_)));
}

#[test]
fn form_render_without_csrf() {
    let form = Form::builder("comment")
        .csrf(false)
        .field("body", vec![])
        .build();
    let rendered = form.render().expect("render");
    let html = rendered.as_html();
    assert!(!html.contains("_token"));
    assert!(html.contains(r#"name="body""#));
}

#[test]
fn form_rejects_bad_csrf() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment").field("body", vec![]).build();
    let request = Request::new(Method::Post, "/").with_body(b"_token=nope&body=x".to_vec());
    let err = form.handle_request(&request, &mgr).expect_err("csrf");
    assert!(matches!(err, FormError::Csrf(_)));
}

#[test]
fn form_get_is_not_submitted() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("comment").build();
    let request = Request::new(Method::Get, "/");
    assert_eq!(
        form.handle_request(&request, &mgr).expect("get"),
        FormStatus::NotSubmitted
    );
}

#[test]
fn field_kinds_render_and_bind() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let choice = ChoiceOptions {
        choices: vec![
            ChoiceEntry::with_label("a", "Alpha"),
            ChoiceEntry::with_label("b", "Beta"),
        ],
        multiple: false,
        expanded: false,
    };
    let mut form = Form::builder("kinds")
        .csrf(false)
        .field_config(
            "note",
            FieldConfig::Textarea(TextareaOptions {
                placeholder: Some("hi".into()),
                rows: Some(3),
            }),
            vec![],
        )
        .field_config(
            "secret",
            FieldConfig::Password(PasswordOptions::default()),
            vec![],
        )
        .field_config("nid", FieldConfig::Hidden(HiddenOptions::default()), vec![])
        .field_config(
            "active",
            FieldConfig::Checkbox(CheckboxOptions::default()),
            vec![],
        )
        .field_config("pick", FieldConfig::Choice(choice), vec![])
        .field_config(
            "qty",
            FieldConfig::Number(NumberOptions {
                min: Some(0),
                max: Some(10),
                step: Some(1),
            }),
            vec![],
        )
        .build();

    assert_eq!(form.fields()[0].kind(), FieldKind::Textarea);
    assert_eq!(form.fields()[3].kind(), FieldKind::Checkbox);
    let html = form.render().expect("render").as_html().to_owned();
    assert!(html.contains("<textarea"));
    assert!(html.contains(r#"type="password""#));
    assert!(html.contains(r#"type="hidden""#));
    assert!(html.contains(r#"type="checkbox""#));
    assert!(html.contains("<select"));
    assert!(html.contains(r#"type="number""#));

    let body = b"note=hello&secret=x&nid=9&active=1&pick=b&qty=4";
    let request = Request::new(Method::Post, "/").with_body(body.to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("note"), Some("hello"));
    assert_eq!(form.get("pick"), Some("b"));
    assert_eq!(form.get("qty"), Some("4"));
    assert_eq!(form.get("active"), Some("1"));
}

#[test]
fn compound_bind_dotted_names_and_unflatten() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("profile")
        .csrf(false)
        .compound("address", |c| {
            c.field("street", vec![])
                .field("city", vec![])
                .compound("geo", |g| g.field("lat", vec![]))
        })
        .build();
    assert!(form.get("address.street").is_some());
    assert!(form.get("address.geo.lat").is_some());

    let body = b"address.street=Main&address.city=Paris&address.geo.lat=48";
    let request = Request::new(Method::Post, "/").with_body(body.to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("address.street"), Some("Main"));
    assert_eq!(form.get("address.geo.lat"), Some("48"));

    let nested = unflatten_form_data(&form.data()).expect("nest");
    assert_eq!(
        nested["address"].as_map().unwrap()["street"].as_str(),
        Some("Main")
    );
    assert_eq!(
        nested["address"].as_map().unwrap()["geo"].as_map().unwrap()["lat"].as_str(),
        Some("48")
    );
}

#[test]
fn collection_bind_indexed_keys_and_prototype() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("order")
        .csrf(false)
        .collection("items", 2, |entry| {
            entry.field("name", vec![]).field_config(
                "qty",
                FieldConfig::Number(NumberOptions::default()),
                vec![],
            )
        })
        .build();
    assert_eq!(form.get("items[0][name]").map(|_| true), Some(true));
    assert_eq!(
        collection_field_name("items", "0", "name"),
        "items[0][name]"
    );
    assert_eq!(
        collection_prototype_field_name("items", "name"),
        format!("items[{COLLECTION_PROTOTYPE_INDEX}][name]")
    );
    let proto = CollectionPrototypeMeta::new("items");
    assert!(
        proto
            .data_attributes_html()
            .contains("data-collection=\"items\"")
    );
    assert_eq!(proto.field_name("name"), "items[__name__][name]");

    let body = b"items%5B0%5D%5Bname%5D=Apple&items%5B0%5D%5Bqty%5D=2&items%5B1%5D%5Bname%5D=Pear&items%5B1%5D%5Bqty%5D=1";
    let request = Request::new(Method::Post, "/").with_body(body.to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("items[0][name]"), Some("Apple"));
    assert_eq!(form.get("items[1][qty]"), Some("1"));

    let indices = collection_indices(&form.data(), "items");
    assert_eq!(indices, vec![0, 1]);

    let nested = unflatten_form_data(&form.data()).expect("nest");
    assert_eq!(
        nested["items"].as_map().unwrap()["0"].as_map().unwrap()["name"].as_str(),
        Some("Apple")
    );
}

#[test]
fn transformers_bool_i64_choice() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let choice = ChoiceOptions {
        choices: vec![ChoiceEntry::new("red"), ChoiceEntry::new("blue")],
        multiple: true,
        expanded: true,
    };
    let choice_transformer = Arc::new(ChoiceToValueTransformer::new(choice.clone()));
    let mut form = Form::builder("prefs")
        .csrf(false)
        .field_transformed(
            "active",
            FieldConfig::Checkbox(CheckboxOptions::default()),
            vec![],
            Arc::new(BoolToStringTransformer),
        )
        .field_transformed(
            "count",
            FieldConfig::Number(NumberOptions::default()),
            vec![],
            Arc::new(I64ToStringTransformer),
        )
        .field_transformed(
            "color",
            FieldConfig::Choice(choice),
            vec![],
            choice_transformer,
        )
        .build();

    let body = b"active=on&count=42&color=red&color=blue";
    let request = Request::new(Method::Post, "/").with_body(body.to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("active"), Some("1"));
    assert_eq!(form.get("count"), Some("42"));
    assert_eq!(form.get("color"), Some("red blue"));

    let html = form.render().expect("render").as_html().to_owned();
    assert!(html.contains("checked"));
    assert!(html.contains(r#"type="checkbox""#));

    let bad = Request::new(Method::Post, "/").with_body(b"count=nope".to_vec());
    let err = form.handle_request(&bad, &mgr).expect_err("i64");
    assert!(matches!(err, FormError::Transform(_)));
}

#[test]
fn csrf_still_default_on_with_compound() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("profile")
        .compound("address", |c| c.field("street", vec![]))
        .build();
    form.prepare_csrf(&mgr).expect("csrf");
    let token = form.render().expect("render").as_html().to_owned();
    assert!(token.contains(r#"name="_token""#));

    let token_value = token
        .split("value=\"")
        .nth(1)
        .expect("value")
        .split('"')
        .next()
        .expect("end")
        .to_owned();
    let body = format!("_token={token_value}&address.street=Main");
    let request = Request::new(Method::Post, "/").with_body(body.into_bytes());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("address.street"), Some("Main"));
}

#[test]
fn unflatten_conflict_errors() {
    let mut flat = HashMap::new();
    flat.insert("address".into(), "x".into());
    flat.insert("address.street".into(), "y".into());
    let err = unflatten_form_data(&flat).expect_err("conflict");
    assert!(matches!(err, FormError::NestedConflict(_)));
}

#[test]
fn field_config_kind_and_accessors() {
    assert_eq!(
        FieldConfig::Text(TextOptions::default()).kind(),
        FieldKind::Text
    );
    assert_eq!(
        FieldConfig::Textarea(TextareaOptions::default()).kind(),
        FieldKind::Textarea
    );
    assert_eq!(
        FieldConfig::Hidden(HiddenOptions::default()).kind(),
        FieldKind::Hidden
    );
    assert_eq!(
        FieldConfig::Password(PasswordOptions::default()).kind(),
        FieldKind::Password
    );
    assert_eq!(
        FieldConfig::Checkbox(CheckboxOptions::default()).kind(),
        FieldKind::Checkbox
    );
    let choice = FieldConfig::Choice(ChoiceOptions::default());
    assert_eq!(choice.kind(), FieldKind::Choice);
    assert!(choice.as_choice().is_some());
    assert!(choice.as_checkbox().is_none());
    assert!(choice.as_number().is_none());

    let checkbox = FieldConfig::Checkbox(CheckboxOptions {
        checked_value: String::from("yes"),
    });
    assert!(checkbox.as_checkbox().is_some());
    assert!(checkbox.as_choice().is_none());
    assert!(checkbox.as_number().is_none());

    let number = FieldConfig::Number(NumberOptions {
        min: Some(1),
        max: Some(9),
        step: Some(1),
    });
    assert_eq!(number.kind(), FieldKind::Number);
    assert!(number.as_number().is_some());
    assert!(number.as_choice().is_none());
    assert!(number.as_checkbox().is_none());

    assert!(FieldConfig::default().as_choice().is_none());
    assert!(FieldConfig::default().as_checkbox().is_none());
    assert!(FieldConfig::default().as_number().is_none());
}

#[test]
fn choice_entry_constructors() {
    let same = ChoiceEntry::new("red");
    assert_eq!(same.value, "red");
    assert_eq!(same.label, "red");
    let labeled = ChoiceEntry::with_label("r", "Rouge");
    assert_eq!(labeled.value, "r");
    assert_eq!(labeled.label, "Rouge");
}

#[test]
fn transformers_error_and_empty_paths() {
    let bool_t = BoolToStringTransformer;
    assert_eq!(bool_t.transform("0").expect("falsy"), "");
    assert_eq!(bool_t.transform("yes").expect("truthy"), "1");
    assert_eq!(bool_t.reverse_transform("YES").expect("case"), "1");

    let i64_t = I64ToStringTransformer;
    assert_eq!(i64_t.transform("").expect("empty model"), "");
    assert_eq!(i64_t.reverse_transform("").expect("empty view"), "");
    assert!(matches!(
        i64_t.transform("nope"),
        Err(FormError::Transform(_))
    ));
    assert_eq!(i64_t.transform("-7").expect("neg"), "-7");

    let single = ChoiceToValueTransformer::new(ChoiceOptions {
        choices: vec![ChoiceEntry::new("a"), ChoiceEntry::new("b")],
        multiple: false,
        expanded: false,
    });
    assert_eq!(single.transform("").expect("empty"), "");
    assert_eq!(single.transform("a").expect("ok"), "a");
    assert!(matches!(
        single.transform("z"),
        Err(FormError::Transform(_))
    ));
    assert!(matches!(
        single.reverse_transform("z"),
        Err(FormError::Transform(_))
    ));

    let multi = ChoiceToValueTransformer::new(ChoiceOptions {
        choices: vec![ChoiceEntry::new("a"), ChoiceEntry::new("b")],
        multiple: true,
        expanded: false,
    });
    assert!(matches!(
        multi.transform("a z"),
        Err(FormError::Transform(_))
    ));
    assert_eq!(multi.transform("a b").expect("ok"), "a b");
    assert_eq!(split_multi("  a  b "), vec!["a", "b"]);
    assert_eq!(join_multi(&[String::from("a"), String::from("b")]), "a b");
}

#[test]
fn nested_data_accessors_and_unflatten_edges() {
    let leaf = NestedData::Value(String::from("x"));
    assert_eq!(leaf.as_str(), Some("x"));
    assert!(leaf.as_map().is_none());

    let map = NestedData::Map(BTreeMap::default());
    assert!(map.as_str().is_none());
    assert!(map.as_map().is_some());

    let mut flat = HashMap::new();
    flat.insert(String::new(), String::from("ignored"));
    flat.insert("solo".into(), String::from("v"));
    let nested = unflatten_form_data(&flat).expect("ok");
    assert_eq!(nested.get("solo").and_then(NestedData::as_str), Some("v"));
    assert!(!nested.contains_key(""));

    let mut indices_flat = HashMap::new();
    indices_flat.insert("items[0][name]".into(), "a".into());
    indices_flat.insert("items[__name__][name]".into(), "proto".into());
    indices_flat.insert("items[2][name]".into(), "c".into());
    indices_flat.insert("other[1][x]".into(), "z".into());
    assert_eq!(collection_indices(&indices_flat, "items"), vec![0, 2]);
}

#[test]
fn compound_and_collection_field_transformed() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("shop")
        .csrf(false)
        .compound("flags", |c| {
            c.field_transformed(
                "active",
                FieldConfig::Checkbox(CheckboxOptions::default()),
                vec![],
                Arc::new(BoolToStringTransformer),
            )
        })
        .collection("items", 1, |entry| {
            entry.field_transformed(
                "qty",
                FieldConfig::Number(NumberOptions::default()),
                vec![],
                Arc::new(I64ToStringTransformer),
            )
        })
        .build();

    assert_eq!(form.fields()[0].config().kind(), FieldKind::Checkbox);
    assert_eq!(form.fields()[1].name(), "items[0][qty]");

    let body = b"flags.active=on&items%5B0%5D%5Bqty%5D=3";
    let request = Request::new(Method::Post, "/").with_body(body.to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("flags.active"), Some("1"));
    assert_eq!(form.get("items[0][qty]"), Some("3"));
}

#[test]
fn choice_expanded_radio_renders_unchecked() {
    let choice = ChoiceOptions {
        choices: vec![
            ChoiceEntry::with_label("a", "Alpha"),
            ChoiceEntry::with_label("b", "Beta"),
        ],
        multiple: false,
        expanded: true,
    };
    let mut form = Form::builder("poll")
        .csrf(false)
        .field_config("pick", FieldConfig::Choice(choice), vec![])
        .build();
    form.set("pick", "a");
    let html = form.render().expect("render").as_html().to_owned();
    assert!(html.contains(r#"type="radio""#));
    assert!(html.contains("checked"));
    assert!(html.contains(r#"value="b""#));
    assert!(!html.contains(r#"value="b" checked"#));
}

#[test]
fn form_patch_method_and_text_placeholder() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("edit")
        .csrf(false)
        .method(Method::Patch)
        .action("/edit")
        .field_config(
            "title",
            FieldConfig::Text(TextOptions {
                placeholder: Some("Title".into()),
            }),
            vec![],
        )
        .field_config(
            "secret",
            FieldConfig::Password(PasswordOptions {
                placeholder: Some("pw".into()),
            }),
            vec![],
        )
        .build();
    let html = form.render().expect("render").as_html().to_owned();
    assert!(html.contains("method=\"PATCH\""));
    assert!(html.contains(r#"placeholder="Title""#));
    assert!(html.contains(r#"placeholder="pw""#));

    let request = Request::new(Method::Patch, "/").with_body(b"title=Hi&secret=x".to_vec());
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("title"), Some("Hi"));
    assert_eq!(form.fields()[0].value(), "Hi");
    assert_eq!(form.fields()[0].config().kind(), FieldKind::Text);
}

#[test]
fn theme_helpers_escape_xss_payloads() {
    let mut form = Form::builder("xss")
        .csrf(false)
        .field("body", vec![])
        .build();
    assert!(form.set("body", r#"<script>alert("x")</script>"#));
    let field = &form.fields()[0];
    let label = form_label(field);
    assert!(label.contains(r#"for="body""#));
    assert!(label.contains(">body</label>"));
    let widget = form_widget(field).expect("widget");
    assert!(!widget.contains("<script>"));
    assert!(widget.contains("&lt;script&gt;"));
    let mut violations = ConstraintViolationList::new();
    violations.add(Violation::new("body", r#"bad <msg> & "quote""#, "NotBlank"));
    let errors = form_errors(field, &violations);
    assert!(errors.contains("form-errors"));
    assert!(errors.contains("&lt;msg&gt;"));
    assert!(!errors.contains("<msg>"));
    let row = form_row(field, &violations).expect("row");
    assert!(row.contains("form-row"));
    assert!(row.contains("&lt;script&gt;"));
}

#[test]
fn form_render_escapes_field_values_xss() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("xss").field("body", vec![]).build();
    form.prepare_csrf(&mgr).expect("csrf");
    assert!(form.set("body", "<img onerror=alert(1) src=x>"));
    let html = form.render().expect("render").as_html().to_owned();
    assert!(html.contains(r#"name="_token""#));
    assert!(!html.contains("<img onerror"));
    assert!(html.contains("&lt;img"));
}

#[test]
fn multipart_file_bind_and_constraints() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let options = FileOptions {
        accept: Some("image/*".into()),
        max_size: 64,
        mime_types: vec!["image/png".into(), "image/*".into()],
        storage: FileStorage::Memory,
    };
    let mut form = Form::builder("upload")
        .csrf(false)
        .field_config("image", FieldConfig::File(options), vec![])
        .field("caption", vec![])
        .build();
    assert_eq!(form.fields()[0].kind(), FieldKind::File);
    let html = form.render().expect("render").as_html().to_owned();
    assert!(html.contains(r#"enctype="multipart/form-data""#));
    assert!(html.contains(r#"type="file""#));
    assert!(html.contains(r#"accept="image/*""#));
    assert_eq!(DEFAULT_MAX_FILE_SIZE, 2 * 1024 * 1024);

    let boundary = "----SerenadeBoundary7";
    let body = [
        format!("--{boundary}\r\n").into_bytes(),
        b"Content-Disposition: form-data; name=\"caption\"\r\n\r\n".to_vec(),
        b"hello\r\n".to_vec(),
        format!("--{boundary}\r\n").into_bytes(),
        b"Content-Disposition: form-data; name=\"image\"; filename=\"pic.png\"\r\n".to_vec(),
        b"Content-Type: image/png\r\n\r\n".to_vec(),
        b"PNGDATA\r\n".to_vec(),
        format!("--{boundary}--\r\n").into_bytes(),
    ]
    .concat();
    let request = Request::new(Method::Post, "/")
        .with_header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .with_body(body);
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    assert_eq!(form.get("caption"), Some("hello"));
    assert_eq!(form.get("image"), Some("pic.png"));
    let file = form.file("image").expect("file");
    assert_eq!(file.filename(), Some("pic.png"));
    assert_eq!(file.content_type(), Some("image/png"));
    assert_eq!(file.as_bytes(), Some(b"PNGDATA".as_slice()));

    let too_big = FileOptions {
        max_size: 3,
        mime_types: vec!["image/png".into()],
        ..FileOptions::default()
    };
    let err = parse_multipart(
        request.body(),
        boundary,
        &HashMap::from([(String::from("image"), too_big)]),
        1024,
    )
    .expect_err("size");
    assert!(matches!(err, FormError::Upload(_)));
}

#[test]
fn multipart_tempfile_storage_and_csrf() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("upload")
        .field_config(
            "doc",
            FieldConfig::File(FileOptions {
                storage: FileStorage::TempFile,
                max_size: 1024,
                mime_types: vec!["text/plain".into()],
                accept: None,
            }),
            vec![],
        )
        .build();
    form.prepare_csrf(&mgr).expect("csrf");
    let token = form
        .render()
        .expect("render")
        .as_html()
        .split("value=\"")
        .nth(1)
        .expect("value")
        .split('"')
        .next()
        .expect("end")
        .to_owned();

    let boundary = "BoundTemp1";
    let body = [
        format!("--{boundary}\r\n").into_bytes(),
        b"Content-Disposition: form-data; name=\"_token\"\r\n\r\n".to_vec(),
        format!("{token}\r\n").into_bytes(),
        format!("--{boundary}\r\n").into_bytes(),
        b"Content-Disposition: form-data; name=\"doc\"; filename=\"note.txt\"\r\n".to_vec(),
        b"Content-Type: text/plain\r\n\r\n".to_vec(),
        b"notes\r\n".to_vec(),
        format!("--{boundary}--\r\n").into_bytes(),
    ]
    .concat();
    let request = Request::new(Method::Post, "/")
        .with_header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .with_body(body);
    assert_eq!(
        form.handle_request(&request, &mgr).expect("bind"),
        FormStatus::Bound
    );
    let file = form.take_file("doc").expect("file");
    assert!(file.path().is_some());
    assert_eq!(file.into_bytes().expect("bytes"), b"notes");
}

#[test]
fn multipart_csrf_rejection() {
    let mgr = HmacCsrfTokenManager::new(b"unit-test-secret-key!!!!!!!!!!!!");
    let mut form = Form::builder("upload")
        .field_config("image", FieldConfig::File(FileOptions::default()), vec![])
        .build();
    let boundary = "BoundBad";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"_token\"\r\n\r\nbad\r\n--{boundary}--\r\n"
    );
    let request = Request::new(Method::Post, "/")
        .with_header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .with_body(body.into_bytes());
    let err = form.handle_request(&request, &mgr).expect_err("csrf");
    assert!(matches!(err, FormError::Csrf(_)));
}

//! Unit tests for `serenade-string`.

use crate::{
    camel_case, kebab_case, pascal_case, pluralize, singularize, slug, snake_case, title_case,
    version,
};

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn slug_basic() {
    assert_eq!(slug("Hello World!"), "hello-world");
    assert_eq!(slug("  --Foo--Bar--  "), "foo-bar");
    assert_eq!(slug(""), "");
}

#[test]
fn case_transforms() {
    assert_eq!(snake_case("HelloWorld"), "hello_world");
    assert_eq!(snake_case("hello-world"), "hello_world");
    assert_eq!(kebab_case("HelloWorld"), "hello-world");
    assert_eq!(camel_case("hello_world"), "helloWorld");
    assert_eq!(pascal_case("hello_world"), "HelloWorld");
    assert_eq!(title_case("hello_world"), "Hello World");
    assert_eq!(camel_case(""), "");
    assert_eq!(pascal_case(""), "");
}

#[test]
fn pluralize_common_rules() {
    assert_eq!(pluralize("cat"), "cats");
    assert_eq!(pluralize("bus"), "buses");
    assert_eq!(pluralize("box"), "boxes");
    assert_eq!(pluralize("buzz"), "buzzes");
    assert_eq!(pluralize("church"), "churches");
    assert_eq!(pluralize("dish"), "dishes");
    assert_eq!(pluralize("baby"), "babies");
    assert_eq!(pluralize("day"), "days");
    assert_eq!(pluralize("leaf"), "leaves");
    assert_eq!(pluralize("knife"), "knives");
    assert_eq!(pluralize("hero"), "heroes");
    assert_eq!(pluralize("person"), "people");
    assert_eq!(pluralize("Child"), "Children");
    assert_eq!(pluralize("SHEEP"), "SHEEP");
    assert_eq!(pluralize(""), "");
}

#[test]
fn singularize_common_rules() {
    assert_eq!(singularize("cats"), "cat");
    assert_eq!(singularize("buses"), "bus");
    assert_eq!(singularize("boxes"), "box");
    assert_eq!(singularize("churches"), "church");
    assert_eq!(singularize("dishes"), "dish");
    assert_eq!(singularize("babies"), "baby");
    assert_eq!(singularize("leaves"), "leaf");
    assert_eq!(singularize("people"), "person");
    assert_eq!(singularize("Children"), "Child");
    assert_eq!(singularize("sheep"), "sheep");
    assert_eq!(singularize("news"), "news");
    assert_eq!(singularize(""), "");
}

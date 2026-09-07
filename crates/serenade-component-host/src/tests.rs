use super::*;
use std::path::PathBuf;

#[test]
fn version_is_nonempty() {
    assert_ne!(version(), "");
}

#[test]
fn default_engine_builds() {
    let _ = default_engine();
}

#[test]
fn load_missing_path_fails() {
    let engine = default_engine();
    let missing = PathBuf::from("/tmp/serenade-component-host-missing.component.wasm");
    match load_component(&engine, &missing) {
        Ok(_) => panic!("missing path should fail"),
        Err(err) => {
            let msg = format!("{err:#}");
            assert!(msg.contains("load component from"), "{msg}");
        }
    }
}

#[test]
fn load_empty_component_from_wat() {
    let engine = default_engine();
    let component = load_component_bytes(&engine, b"(component)").expect("empty component");
    let linker = empty_linker::<()>(&engine);
    let mut store = store_with_data(&engine, ());
    linker
        .instantiate(&mut store, &component)
        .expect("instantiate with empty linker");
}

#[test]
fn empty_linker_denies_missing_imports() {
    let engine = default_engine();
    // Component that imports a host function; empty linker supplies nothing.
    let wat = br#"(component
  (import "missing" (func))
)"#;
    let component = load_component_bytes(&engine, wat).expect("component with import");
    let linker = empty_linker::<()>(&engine);
    let mut store = store_with_data(&engine, ());
    assert!(
        linker.instantiate(&mut store, &component).is_err(),
        "empty linker must deny missing imports"
    );
}

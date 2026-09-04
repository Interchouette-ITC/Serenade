use serenade_di::{ContainerBuilder, ParameterBag};

use super::{load_dotenv, load_packages, load_packages_for_env, version, Config};

#[test]
fn version_matches_workspace() {
    assert_eq!(version(), "0.1.0");
}

#[test]
fn yaml_and_toml_merge_with_env_and_parameters() {
    std::env::set_var("SERENADE_CONFIG_TEST_HOST", "db.example");
    let defaults = Config::from_yaml(
        "
database:
  host: localhost
  port: 5432
  name: serenade
app:
  debug: true
",
    )
    .unwrap();
    let overlay = Config::from_toml(concat!(
        "[database]\n",
        "host = \"$",
        "{SERENADE_CONFIG_TEST_HOST}\"\n",
        "name = \"$",
        "{SERENADE_CONFIG_TEST_NAME:-shop}\"\n",
        "[app]\n",
        "debug = false\n",
    ))
    .unwrap();
    let loaded = defaults
        .merged(&overlay)
        .interpolate_env()
        .expect("interpolate");
    let params = loaded.parameters();
    assert_eq!(
        params.get("database.host").map(String::as_str),
        Some("db.example")
    );
    assert_eq!(
        params.get("database.port").map(String::as_str),
        Some("5432")
    );
    assert_eq!(
        params.get("database.name").map(String::as_str),
        Some("shop")
    );
    assert_eq!(params.get("app.debug").map(String::as_str), Some("false"));

    let mut bag = ParameterBag::new();
    loaded.apply_to(&mut bag);
    assert_eq!(bag.get("database.host").unwrap(), "db.example");

    let mut builder = ContainerBuilder::new();
    loaded.apply_to(builder.parameters_mut());
    assert!(builder.parameters().contains("database.name"));
}

#[test]
fn missing_env_without_default_is_error() {
    let config = Config::from_yaml(concat!("url: $", "{SERENADE_CONFIG_TEST_MISSING}\n")).unwrap();
    let error = config.interpolate_env().unwrap_err();
    assert!(error.to_string().contains("SERENADE_CONFIG_TEST_MISSING"));
}

#[test]
fn load_packages_merges_sorted_files() {
    let dir = unique_temp_dir("packages");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("00-framework.yaml"),
        concat!(
            "framework:\n  secret: $",
            "{UNSET:-dev-secret}\n  name: kernel\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("10-app.toml"), "[framework]\nname = \"shop\"\n").unwrap();
    let loaded = load_packages(&dir).unwrap().interpolate_env().unwrap();
    let params = loaded.parameters();
    assert_eq!(
        params.get("framework.name").map(String::as_str),
        Some("shop")
    );
    assert_eq!(
        params.get("framework.secret").map(String::as_str),
        Some("dev-secret")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn load_packages_for_env_applies_overlay_directory() {
    let dir = unique_temp_dir("packages-env");
    std::fs::create_dir_all(dir.join("dev")).unwrap();
    std::fs::write(
        dir.join("framework.toml"),
        "[framework]\nname = \"base\"\ndebug = false\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("dev").join("framework.toml"),
        "[framework]\ndebug = true\n",
    )
    .unwrap();
    let loaded = load_packages_for_env(&dir, "dev").unwrap();
    let params = loaded.parameters();
    assert_eq!(
        params.get("framework.name").map(String::as_str),
        Some("base")
    );
    assert_eq!(
        params.get("framework.debug").map(String::as_str),
        Some("true")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn load_dotenv_sets_missing_vars_in_order() {
    let dir = unique_temp_dir("dotenv");
    std::fs::create_dir_all(&dir).unwrap();
    std::env::remove_var("SERENADE_DOTENV_A");
    std::env::remove_var("SERENADE_DOTENV_B");
    std::fs::write(
        dir.join(".env"),
        "SERENADE_DOTENV_A=from-env\nSERENADE_DOTENV_B=base\n",
    )
    .unwrap();
    std::fs::write(dir.join(".env.local"), "SERENADE_DOTENV_B=local\n").unwrap();
    std::fs::write(dir.join(".env.dev"), "SERENADE_DOTENV_B=dev\n").unwrap();
    load_dotenv(&dir, "dev").unwrap();
    assert_eq!(std::env::var("SERENADE_DOTENV_A").unwrap(), "from-env");
    // Later dotenv files override earlier ones; process env still wins.
    assert_eq!(std::env::var("SERENADE_DOTENV_B").unwrap(), "dev");
    std::env::remove_var("SERENADE_DOTENV_A");
    std::env::remove_var("SERENADE_DOTENV_B");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn load_dotenv_skips_local_in_prod() {
    let dir = unique_temp_dir("dotenv-prod");
    std::fs::create_dir_all(&dir).unwrap();
    std::env::remove_var("SERENADE_DOTENV_PROD");
    std::fs::write(dir.join(".env"), "SERENADE_DOTENV_PROD=base\n").unwrap();
    std::fs::write(dir.join(".env.local"), "SERENADE_DOTENV_PROD=local\n").unwrap();
    load_dotenv(&dir, "prod").unwrap();
    assert_eq!(std::env::var("SERENADE_DOTENV_PROD").unwrap(), "base");
    std::env::remove_var("SERENADE_DOTENV_PROD");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn section_returns_nested_object_or_empty() {
    let config = Config::from_toml(
        r#"
[framework]
secret = "x"

[demo]
name = "y"
"#,
    )
    .unwrap();
    let framework = config.section("framework");
    assert_eq!(
        framework.parameters().get("secret").map(String::as_str),
        Some("x")
    );
    assert!(config.section("missing").parameters().is_empty());
    let scalar = Config::from_yaml("framework: 1\n").unwrap();
    assert!(scalar.section("framework").parameters().is_empty());
    assert!(scalar.value().is_object());
}

#[test]
fn from_path_rejects_unsupported_and_reads_yaml() {
    let dir = unique_temp_dir("from-path");
    std::fs::create_dir_all(&dir).unwrap();
    let yaml = dir.join("app.yaml");
    std::fs::write(&yaml, "app:\n  name: shop\n").unwrap();
    let loaded = Config::from_path(&yaml).unwrap();
    assert_eq!(
        loaded.parameters().get("app.name").map(String::as_str),
        Some("shop")
    );
    let bad = dir.join("app.json");
    std::fs::write(&bad, "{}").unwrap();
    let err = Config::from_path(&bad).unwrap_err();
    assert!(
        err.to_string().contains("Unsupported")
            || err.to_string().contains("unsupported")
            || matches!(err, super::ConfigError::UnsupportedFormat { .. })
    );
    let root = Config::from_yaml("- just\n- a\n- list\n").unwrap_err();
    assert!(matches!(root, super::ConfigError::InvalidRoot));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn interpolate_arrays_and_rejects_bad_idents() {
    let with_array = Config::from_yaml(concat!(
        "hosts:\n",
        "  - $",
        "{SERENADE_CONFIG_ARRAY:-localhost}\n",
    ))
    .unwrap()
    .interpolate_env()
    .unwrap();
    let hosts = with_array
        .value()
        .get("hosts")
        .and_then(|value| value.as_array())
        .expect("hosts array");
    assert_eq!(hosts[0].as_str(), Some("localhost"));
    let bad = Config::from_yaml(concat!("x: $", "{1bad}\n")).unwrap();
    assert!(bad.interpolate_env().is_err());
    let empty = Config::from_yaml(concat!("x: $", "{}\n")).unwrap();
    assert!(empty.interpolate_env().is_err());
}

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "serenade-config-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ))
}

use serenade_di::{ContainerBuilder, ParameterBag};

use super::{load_packages, version, Config};

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
    let dir = std::env::temp_dir().join(format!(
        "serenade-config-packages-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
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

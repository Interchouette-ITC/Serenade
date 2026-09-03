//! Scaffolded Serenade application (`serenade new`).

use std::path::PathBuf;

use serenade_bundle::{build_container, BundleError, FrameworkBundle, FrameworkExtension};
use serenade_kernel::{App, Application, Environment};

fn main() -> Result<(), BundleError> {
    let env_name = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_owned());
    let environment = Environment::from_name(&env_name).map_err(|error| BundleError::Extension {
        alias: "app",
        message: error.to_string(),
    })?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    serenade_config::load_dotenv(&root, environment.as_str()).map_err(|error| {
        BundleError::Extension {
            alias: "app",
            message: error.to_string(),
        }
    })?;

    let mut app = App::new(environment.clone());
    app.register_bundle(FrameworkBundle)?;
    app.boot()?;

    let packages = root.join("config/packages");
    let (_config, container) = build_container(
        Some(packages.as_path()),
        environment.as_str(),
        &[&FrameworkExtension],
    )?;

    println!("bundles: {:?}", app.kernel().bundle_names());
    let _ = container;

    app.shutdown()?;
    Ok(())
}

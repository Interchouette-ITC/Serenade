//! Sample console entry for the demo app (`bin/console` analogue).

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use serenade_bundle::{
    build_container, FrameworkBundle, FrameworkExtension, CONSOLE_APPLICATION_SERVICE,
};
use serenade_console::Application;
use serenade_kernel::{App, Application as KernelApp, Environment};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let env_name = std::env::var("APP_ENV").unwrap_or_else(|_| "dev".to_owned());
    let environment = Environment::from_name(&env_name)?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    serenade_config::load_dotenv(&root, environment.as_str())?;

    let mut app = App::new(environment.clone());
    app.register_bundle(FrameworkBundle)?;
    app.boot()?;

    let packages = root.join("config/packages");
    let (_config, container) = build_container(
        Some(packages.as_path()),
        environment.as_str(),
        &[&FrameworkExtension],
    )?;
    let container = Arc::new(container);
    let console = container.get_as::<Application>(CONSOLE_APPLICATION_SERVICE)?;
    let argv: Vec<String> = std::env::args().collect();
    console.run_with(argv, Some(Arc::clone(&container)))?;
    app.shutdown()?;
    Ok(())
}

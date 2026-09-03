//! Console entry (`bin/console` analogue).

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
    let mut app = App::new(environment);
    app.register_bundle(FrameworkBundle)?;
    app.boot()?;

    let packages = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config/packages");
    let (_config, container) = build_container(Some(packages.as_path()), &[&FrameworkExtension])?;
    let container = Arc::new(container);
    let console = container.get_as::<Application>(CONSOLE_APPLICATION_SERVICE)?;
    let argv: Vec<String> = std::env::args().collect();
    console.run_with(argv, Some(Arc::clone(&container)))?;
    app.shutdown()?;
    Ok(())
}

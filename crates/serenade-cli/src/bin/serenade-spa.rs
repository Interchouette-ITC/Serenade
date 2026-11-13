//! Build registered Serenade SPA apps with Trunk.

use clap::Parser;
use serenade_cli::mode::{discover_apps, find_examples_root, select_apps};

#[derive(Debug, Parser)]
#[command(
    name = "serenade-spa",
    about = "Build a registered Serenade SPA app with Trunk"
)]
struct Cli {
    /// Build every app registered with mode = "spa".
    #[arg(long, conflicts_with = "app")]
    all: bool,
    /// Build the app named APP (defaults to demo-spa).
    app: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = find_examples_root(&std::env::current_dir()?)?;
    let apps = discover_apps(&root.join("examples"))?;
    let selected = select_apps(&apps, "spa", "demo-spa", cli.all, cli.app.as_deref())?;

    for app in selected {
        println!("Building {} ({})", app.name, app.directory.display());
        serenade_cli::trunk::build(&app.directory, &[]).map_err(|error| {
            anyhow::anyhow!("failed to build SPA app `{}`: {error:#}", app.name)
        })?;
        let dist = serenade_cli::html_output::dist_directory_for_build(&app.directory, &[])?;
        let minified = serenade_cli::html_output::minify_html_files(&dist)?;
        println!("Minified {minified} HTML document(s) for {}", app.name);
    }

    Ok(())
}

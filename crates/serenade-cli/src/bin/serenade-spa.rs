//! Build registered Serenade SPA apps with Trunk.

use std::process::Command;

use anyhow::{Context, bail};
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
        let status = Command::new("trunk")
            .arg("build")
            .current_dir(&app.directory)
            .status()
            .with_context(|| format!("failed to start Trunk for app `{}`", app.name))?;
        if !status.success() {
            bail!(
                "Trunk failed to build app `{}` with status {status}",
                app.name
            );
        }
        let dist = serenade_cli::html_output::dist_directory(&app.directory)?;
        let minified = serenade_cli::html_output::minify_html_files(&dist)?;
        println!("Minified {minified} HTML document(s) for {}", app.name);
    }

    Ok(())
}

//! Build and run registered Serenade Universal apps.

use std::process::Command;

use anyhow::{Context, anyhow, bail};
use clap::Parser;
use serenade_cli::mode::{discover_apps, find_examples_root, select_apps};

#[derive(Debug, Parser)]
#[command(name = "serenade-universal", about = "Build and run a Universal app")]
struct Cli {
    /// Run every app registered with mode = "universal".
    #[arg(long, conflicts_with = "app")]
    all: bool,
    /// App name to run (defaults to demo-universal).
    app: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = find_examples_root(&std::env::current_dir()?)?;
    let apps = discover_apps(&root.join("examples"))?;
    let selected = select_apps(
        &apps,
        "universal",
        "demo-universal",
        cli.all,
        cli.app.as_deref(),
    )?;

    for app in selected {
        let front = app
            .directory
            .parent()
            .ok_or_else(|| anyhow!("backend app has no parent: {}", app.directory.display()))?
            .join("demo-front");
        let back_manifest = app.directory.join("Cargo.toml");
        println!("Building {} frontend with Trunk", app.name);
        let status = Command::new("trunk")
            .arg("build")
            .current_dir(&front)
            .status()
            .context("failed to start Trunk for Universal frontend")?;
        if !status.success() {
            bail!("Trunk failed to build Universal app `{}`", app.name);
        }
        let dist = serenade_cli::html_output::dist_directory(&front)?;
        let minified = serenade_cli::html_output::minify_html_files(&dist)?;
        println!(
            "Minified {minified} frontend HTML document(s) for {}",
            app.name
        );

        println!("Starting {} backend", app.name);
        let status = Command::new("cargo")
            .args(["run", "--manifest-path"])
            .arg(back_manifest)
            .current_dir(&root)
            .status()
            .with_context(|| format!("failed to start backend for `{}`", app.name))?;
        if !status.success() {
            bail!(
                "Universal backend `{}` exited with status {status}",
                app.name
            );
        }
    }

    Ok(())
}

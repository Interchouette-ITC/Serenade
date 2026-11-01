//! Build registered Serenade SSG apps with Trunk and prerender their routes.

use std::process::Command;

use anyhow::{Context, bail};
use clap::Parser;
use serenade_cli::mode::{discover_apps, find_examples_root, select_apps};

#[derive(Debug, Parser)]
#[command(
    name = "serenade-ssg",
    about = "Build a registered Serenade SSG app with Trunk"
)]
struct Cli {
    /// Build every app registered with mode = "ssg".
    #[arg(long, conflicts_with = "app")]
    all: bool,
    /// Build the app named APP (defaults to demo-ssg).
    app: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = find_examples_root(&std::env::current_dir()?)?;
    let apps = discover_apps(&root.join("examples"))?;
    let selected = select_apps(&apps, "ssg", "demo-ssg", cli.all, cli.app.as_deref())?;

    for app in selected {
        println!("Building {} hydration bundle with Trunk", app.name);
        let trunk_status = Command::new("trunk")
            .args(["build", "--features", "hydrate"])
            .current_dir(&app.directory)
            .status()
            .with_context(|| format!("failed to start Trunk for app `{}`", app.name))?;
        if !trunk_status.success() {
            bail!("Trunk failed to build SSG app `{}`", app.name);
        }

        println!("Prerendering {} routes", app.name);
        let status = Command::new("cargo")
            .args(["run", "--locked", "--manifest-path"])
            .arg(app.directory.join("Cargo.toml"))
            .args([
                "--bin",
                "prerender",
                "--no-default-features",
                "--features",
                "ssr",
            ])
            .current_dir(&root)
            .status()
            .with_context(|| format!("failed to start prerenderer for `{}`", app.name))?;
        if !status.success() {
            bail!(
                "SSG prerenderer for app `{}` exited with status {status}",
                app.name
            );
        }
        let dist = serenade_cli::html_output::dist_directory(&app.directory)?;
        let minified = serenade_cli::html_output::minify_html_files(&dist)?;
        println!("Minified {minified} HTML document(s) for {}", app.name);
    }

    Ok(())
}

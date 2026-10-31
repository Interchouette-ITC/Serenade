//! Run registered Serenade HTML apps.

use std::process::{Child, Command};

use anyhow::{Context, bail};
use clap::Parser;
use serenade_cli::mode::{discover_apps, find_examples_root, select_apps};

#[derive(Debug, Parser)]
#[command(name = "serenade-html", about = "Run a registered Serenade HTML app")]
struct Cli {
    /// Run every app registered with mode = "html".
    #[arg(long, conflicts_with = "app")]
    all: bool,
    /// Run the app named APP (defaults to demo-html).
    app: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let root = find_examples_root(&std::env::current_dir()?)?;
    let apps = discover_apps(&root.join("examples"))?;
    let selected = select_apps(&apps, "html", "demo-html", cli.all, cli.app.as_deref())?;
    let mut children = Vec::with_capacity(selected.len());

    for app in selected {
        println!("Serving {} ({})", app.name, app.directory.display());
        let child = match Command::new("cargo")
            .args(["run", "--manifest-path"])
            .arg(app.directory.join("Cargo.toml"))
            .current_dir(&root)
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                terminate_remaining(&mut children);
                return Err(error).with_context(|| format!("failed to start app `{}`", app.name));
            }
        };
        children.push((app.name.as_str(), child));
    }

    wait_for_apps(children)
}

fn wait_for_apps(mut children: Vec<(&str, Child)>) -> anyhow::Result<()> {
    while !children.is_empty() {
        let mut index = 0;
        while index < children.len() {
            match children[index]
                .1
                .try_wait()
                .context("failed to check app process")?
            {
                Some(status) if status.success() => {
                    children.remove(index);
                }
                Some(status) => {
                    let (name, _failed_child) = children.remove(index);
                    terminate_remaining(&mut children);
                    bail!("HTML app `{name}` exited with status {status}");
                }
                None => index += 1,
            }
        }
        if !children.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    Ok(())
}

fn terminate_remaining(children: &mut [(&str, Child)]) {
    for (name, child) in children.iter_mut() {
        if let Err(error) = child.kill() {
            eprintln!("failed to stop HTML app `{name}`: {error}");
        }
    }
    for (name, child) in children.iter_mut() {
        if let Err(error) = child.wait() {
            eprintln!("failed to wait for HTML app `{name}`: {error}");
        }
    }
}

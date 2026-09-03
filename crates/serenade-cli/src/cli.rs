//! Clap command tree + cling handlers for the `serenade` binary.

use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Context};
use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate, Shell};
use cling::prelude::*;
use serenade_cli::recipe::print_hints;
use serenade_cli::{
    apply_recipe, list_recipes, run_recipe_tui, scaffold_app, ApplyOptions, NewOptions,
};

/// Top-level Serenade scaffolding CLI.
#[derive(Run, Parser, Debug, Clone)]
#[command(
    name = "serenade",
    about = "Serenade scaffolding CLI (Cargo stays the package manager)",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Run, Subcommand, Debug, Clone)]
enum Commands {
    /// Create a new Serenade application skeleton.
    New(NewArgs),
    /// Flex-like recipe commands.
    #[command(subcommand)]
    Recipe(RecipeCommands),
    /// Generate shell completions (`clap_complete`).
    Completion(CompletionArgs),
    /// Generate a man page (`clap_mangen`).
    Man(ManArgs),
    /// Guided TUI to pick and apply a recipe.
    Tui(TuiArgs),
}

#[derive(Run, Collect, Args, Debug, Clone)]
#[cling(run = "cmd_new")]
struct NewArgs {
    /// Package / directory name.
    name: String,
    /// Parent directory (default: current directory).
    #[arg(long, default_value = ".")]
    path: PathBuf,
    /// Replace an existing non-empty destination.
    #[arg(long)]
    force: bool,
}

#[derive(Run, Subcommand, Debug, Clone)]
enum RecipeCommands {
    /// List embedded recipes.
    #[cling(run = "cmd_recipe_list")]
    List,
    /// Apply a recipe into an application tree.
    Apply(ApplyArgs),
}

#[derive(Run, Collect, Args, Debug, Clone)]
#[cling(run = "cmd_recipe_apply")]
struct ApplyArgs {
    /// Recipe id (`framework`, `security`, …).
    id: String,
    /// Application root (default: `.`).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Overwrite existing recipe files.
    #[arg(long)]
    force: bool,
    /// Do not run `cargo add` for declared dependencies.
    #[arg(long)]
    no_cargo: bool,
}

#[derive(Run, Collect, Args, Debug, Clone)]
#[cling(run = "cmd_tui")]
struct TuiArgs {
    /// Application root (default: `.`).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Overwrite existing recipe files.
    #[arg(long)]
    force: bool,
    /// Do not run `cargo add` for declared dependencies.
    #[arg(long)]
    no_cargo: bool,
}

#[derive(Run, Collect, Args, Debug, Clone)]
#[cling(run = "cmd_completion")]
struct CompletionArgs {
    /// Shell to generate completions for.
    shell: ShellKind,
}

#[derive(Run, Collect, Args, Debug, Clone)]
#[cling(run = "cmd_man")]
struct ManArgs {
    /// Output path (default: stdout).
    #[arg(long)]
    output: Option<PathBuf>,
}

/// Shells supported by `serenade completion`.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ShellKind {
    Bash,
    Elvish,
    Fish,
    Powershell,
    Zsh,
}

impl From<ShellKind> for Shell {
    fn from(value: ShellKind) -> Self {
        match value {
            ShellKind::Bash => Self::Bash,
            ShellKind::Elvish => Self::Elvish,
            ShellKind::Fish => Self::Fish,
            ShellKind::Powershell => Self::PowerShell,
            ShellKind::Zsh => Self::Zsh,
        }
    }
}

fn cmd_new(args: &NewArgs) -> anyhow::Result<()> {
    let root = scaffold_app(
        &args.name,
        &NewOptions {
            path: args.path.clone(),
            force: args.force,
        },
    )
    .map_err(|error| anyhow!("{error}"))?;
    println!("created {}", root.display());
    println!("next: cd {} && cargo run", root.display());
    Ok(())
}

fn cmd_recipe_list() {
    for (id, description) in list_recipes() {
        println!("{id}\t{description}");
    }
}

fn cmd_recipe_apply(args: &ApplyArgs) -> anyhow::Result<()> {
    let recipe = apply_recipe(
        &args.id,
        &ApplyOptions {
            root: args.root.clone(),
            force: args.force,
            no_cargo: args.no_cargo,
        },
    )
    .map_err(|error| anyhow!("{error}"))?;
    println!(
        "applied recipe `{}` into {}",
        recipe.id,
        args.root.display()
    );
    print_hints(&recipe);
    Ok(())
}

fn cmd_tui(args: &TuiArgs) -> anyhow::Result<()> {
    match run_recipe_tui(&ApplyOptions {
        root: args.root.clone(),
        force: args.force,
        no_cargo: args.no_cargo,
    }) {
        Ok(_) | Err(serenade_cli::CliError::Cancelled) => Ok(()),
        Err(error) => Err(anyhow!("{error}")),
    }
}

fn cmd_completion(args: &CompletionArgs) {
    let mut command = Cli::command();
    let name = command.get_name().to_owned();
    generate(
        Shell::from(args.shell),
        &mut command,
        name,
        &mut std::io::stdout(),
    );
}

fn cmd_man(args: &ManArgs) -> anyhow::Result<()> {
    let command = Cli::command();
    let man = clap_mangen::Man::new(command);
    let mut buffer = Vec::new();
    man.render(&mut buffer)
        .context("failed to render man page")?;
    if let Some(path) = &args.output {
        std::fs::write(path, &buffer)
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        std::io::stdout()
            .write_all(&buffer)
            .context("failed to write man page to stdout")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_command_tree_is_consistent() {
        Cli::command().debug_assert();
    }
}

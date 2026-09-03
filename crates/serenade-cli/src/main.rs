//! `serenade` binary: Flex-like recipes and app scaffolding.
//!
//! Cargo remains the package manager. This CLI only writes files and may invoke
//! `cargo add` when applying recipes.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serenade_cli::recipe::print_hints;
use serenade_cli::{apply_recipe, list_recipes, scaffold_app, ApplyOptions, CliError, NewOptions};

#[derive(Debug, Parser)]
#[command(
    name = "serenade",
    about = "Serenade scaffolding CLI (Cargo stays the package manager)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a new Serenade application skeleton.
    New {
        /// Package / directory name.
        name: String,
        /// Parent directory (default: current directory).
        #[arg(long, default_value = ".")]
        path: PathBuf,
        /// Replace an existing non-empty destination.
        #[arg(long)]
        force: bool,
    },
    /// Flex-like recipe commands.
    Recipe {
        #[command(subcommand)]
        command: RecipeCommands,
    },
}

#[derive(Debug, Subcommand)]
enum RecipeCommands {
    /// List embedded recipes.
    List,
    /// Apply a recipe into an application tree.
    Apply {
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
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name, path, force } => {
            let root = scaffold_app(&name, &NewOptions { path, force })?;
            println!("created {}", root.display());
            println!("next: cd {} && cargo run", root.display());
        }
        Commands::Recipe {
            command: RecipeCommands::List,
        } => {
            for (id, description) in list_recipes() {
                println!("{id}\t{description}");
            }
        }
        Commands::Recipe {
            command:
                RecipeCommands::Apply {
                    id,
                    root,
                    force,
                    no_cargo,
                },
        } => {
            let recipe = apply_recipe(
                &id,
                &ApplyOptions {
                    root: root.clone(),
                    force,
                    no_cargo,
                },
            )?;
            println!("applied recipe `{}` into {}", recipe.id, root.display());
            print_hints(&recipe);
        }
    }
    Ok(())
}

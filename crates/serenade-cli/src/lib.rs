//! Serenade CLI library: recipes and app scaffolding.
//!
//! The `serenade` binary is scaffolding only. In-app `bin/console` stays in
//! `serenade-console`. Cargo remains the package manager.

pub mod error;
pub mod new;
pub mod recipe;

pub use error::CliError;
pub use new::{scaffold_app, NewOptions};
pub use recipe::{apply_recipe, list_recipes, load_recipe, ApplyOptions, Recipe};

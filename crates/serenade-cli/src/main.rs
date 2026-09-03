//! `serenade` binary: Flex-like recipes and app scaffolding.
//!
//! Built with clap (parse) + cling (handlers) + `clap_complete` / `clap_mangen`.
//! Cargo remains the package manager.

mod cli;

use cling::prelude::*;

use cli::Cli;

#[tokio::main]
async fn main() -> ClingFinished<Cli> {
    Cling::parse_and_run().await
}

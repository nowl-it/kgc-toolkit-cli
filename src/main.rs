use clap::Parser;
use tracing::info;

mod cli;
mod config;
mod core;
mod deps;
mod proxy;
mod tui;
mod utils;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    utils::logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Download(args) => cli::download::run(args).await?,
        Commands::C2u(args) => cli::c2u::run(args).await?,
        Commands::Compare(args) => cli::compare::run(args).await?,
        Commands::Proxy(args) => cli::proxy::run(args).await?,
        Commands::Unity(args) => cli::unity::run(args).await?,
        Commands::Config(args) => cli::config::run(args).await?,
        Commands::Deps(args) => cli::deps::run(args).await?,
        Commands::Tui => tui::run().await?,
    }

    Ok(())
}

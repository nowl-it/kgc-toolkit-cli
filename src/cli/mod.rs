//! CLI command definitions using clap

use clap::{Parser, Subcommand};

pub mod c2u;
pub mod compare;
pub mod config;
pub mod deps;
pub mod download;
pub mod proxy;
pub mod unity;

/// KGC Toolkit - CLI for King God Castle asset management
#[derive(Parser)]
#[command(name = "kgc")]
#[command(author = "nowl-it")]
#[command(version)]
#[command(about = "CLI toolkit for King God Castle asset management and analysis")]
#[command(long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download XAPK from APKPure
    Download(download::DownloadArgs),

    /// Convert XAPK to Unity project
    #[command(name = "c2u")]
    C2u(c2u::C2uArgs),

    /// Compare two Unity project versions
    Compare(compare::CompareArgs),

    /// MITM proxy for API traffic capture
    Proxy(proxy::ProxyArgs),

    /// Unity asset parsing utilities
    Unity(unity::UnityArgs),

    /// XML config management
    Config(config::ConfigArgs),

    /// Dependency management (tools)
    Deps(deps::DepsArgs),

    /// Interactive TUI mode
    Tui,
}

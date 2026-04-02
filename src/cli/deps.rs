//! Dependency management commands

use clap::{Args, Subcommand};
use console::style;

#[derive(Args)]
pub struct DepsArgs {
    #[command(subcommand)]
    pub command: DepsCommand,
}

#[derive(Subcommand)]
pub enum DepsCommand {
    /// Check status of all tools
    Check,

    /// Install a specific tool
    Install {
        /// Tool name (AssetRipper, Il2CppDumper)
        tool: String,
    },

    /// Install all required tools
    InstallAll,

    /// Show tool paths
    Paths,
}

pub async fn run(args: DepsArgs) -> anyhow::Result<()> {
    match args.command {
        DepsCommand::Check => {
            println!("{} Checking dependencies...", style("[Deps]").cyan().bold());

            let status = crate::deps::check_all()?;

            for (tool, available) in &status {
                let icon = if *available {
                    style("✓").green()
                } else {
                    style("✗").red()
                };
                let status_text = if *available {
                    style("installed").green()
                } else {
                    style("missing").red()
                };
                println!("  {} {} - {}", icon, tool, status_text);
            }
        }

        DepsCommand::Install { tool } => {
            println!(
                "{} Installing {}...",
                style("[Deps]").cyan().bold(),
                tool
            );

            crate::deps::install(&tool, |progress, msg| {
                println!("  [{}%] {}", progress, msg);
            })
            .await?;

            println!(
                "{} {} installed successfully",
                style("[✓]").green().bold(),
                tool
            );
        }

        DepsCommand::InstallAll => {
            println!("{} Installing all tools...", style("[Deps]").cyan().bold());

            crate::deps::install_all(|tool, progress, msg| {
                println!("  [{}] [{}%] {}", tool, progress, msg);
            })
            .await?;

            println!(
                "{} All tools installed",
                style("[✓]").green().bold()
            );
        }

        DepsCommand::Paths => {
            println!("{} Tool paths:", style("[Deps]").cyan().bold());

            let paths = crate::deps::get_paths();
            for (tool, path) in &paths {
                let status = if path.exists() {
                    style("✓").green()
                } else {
                    style("✗").red()
                };
                println!("  {} {} → {}", status, tool, path.display());
            }
        }
    }

    Ok(())
}

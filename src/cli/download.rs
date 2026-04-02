//! Download XAPK from APKPure

use clap::Args;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

#[derive(Args)]
pub struct DownloadArgs {
    /// Specific version to download (e.g., "165.0.00")
    #[arg(short, long)]
    pub version: Option<String>,

    /// List available versions instead of downloading
    #[arg(short, long)]
    pub list: bool,

    /// Output directory for downloaded XAPK
    #[arg(short, long, default_value = ".")]
    pub output: PathBuf,
}

pub async fn run(args: DownloadArgs) -> anyhow::Result<()> {
    use crate::core::downloader;
    use crate::core::kgc::PACKAGE_ID;

    if args.list {
        println!("{} Fetching available versions...", style("[KGC]").cyan().bold());

        let versions = downloader::list_versions(PACKAGE_ID).await?;

        if versions.is_empty() {
            println!("{} No versions found", style("[!]").yellow());
        } else {
            println!("{} Available versions:", style("[✓]").green().bold());
            for v in &versions {
                println!("  - {}", v);
            }
        }

        return Ok(());
    }

    // Download XAPK
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}% {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    println!(
        "{} Downloading King God Castle {}...",
        style("[KGC]").cyan().bold(),
        args.version.as_deref().unwrap_or("(latest)")
    );

    let result = downloader::download_xapk(
        PACKAGE_ID,
        args.version.as_deref(),
        &args.output,
        |progress, msg| {
            pb.set_position(progress as u64);
            pb.set_message(msg.to_string());
        },
    )
    .await?;

    pb.finish_with_message("Download complete!");

    println!(
        "{} Downloaded: {}",
        style("[✓]").green().bold(),
        result.display()
    );

    Ok(())
}

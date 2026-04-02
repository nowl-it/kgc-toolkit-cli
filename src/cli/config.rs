//! XML config management commands

use clap::{Args, Subcommand};
use console::style;
use std::path::PathBuf;

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Fetch XML bundle from KGC CDN
    Fetch {
        /// Custom output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Extract XML files from bundle
    Extract {
        /// Path to bundle file
        bundle_path: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,
    },

    /// List extracted XML config files
    List {
        /// Directory containing XML files
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },

    /// Read and display a specific XML file
    Read {
        /// XML file name or path
        file: String,
    },
}

pub async fn run(args: ConfigArgs) -> anyhow::Result<()> {
    match args.command {
        ConfigCommand::Fetch { output } => {
            println!(
                "{} Fetching XML bundle from proxy DB...",
                style("[Config]").cyan().bold()
            );

            let result = crate::core::xml_config::download_xml_bundle_from_source(output.as_deref()).await?;

            println!(
                "{} Downloaded: {}",
                style("[✓]").green().bold(),
                result.bundle_path
            );
            println!("  Base URL: {}", result.base_url);
            println!("  Download URL: {}", result.download_url);
        }

        ConfigCommand::Extract { bundle_path, output } => {
            println!(
                "{} Extracting XML files...",
                style("[Config]").cyan().bold()
            );

            let extracted = crate::core::xml_config::extract_xml_files(&bundle_path, &output)?;

            println!(
                "{} Extracted {} XML files to {}",
                style("[✓]").green().bold(),
                extracted.len(),
                output.display()
            );
        }

        ConfigCommand::List { dir } => {
            let xml_dir = dir.unwrap_or_else(|| crate::core::xml_config::default_xml_dir());

            let files = crate::core::xml_config::list_files(&xml_dir)?;

            if files.is_empty() {
                println!("{} No XML files found", style("[!]").yellow());
            } else {
                println!("{} XML files:", style("[Config]").cyan().bold());
                for file in &files {
                    println!("  - {}", file);
                }
            }
        }

        ConfigCommand::Read { file } => {
            let content = crate::core::xml_config::read_file(&file)?;
            println!("{}", content);
        }
    }

    Ok(())
}

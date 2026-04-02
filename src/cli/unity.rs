//! Unity asset parsing commands

use clap::{Args, Subcommand};
use console::style;
use std::path::PathBuf;

#[derive(Args)]
pub struct UnityArgs {
    #[command(subcommand)]
    pub command: UnityCommand,
}

#[derive(Subcommand)]
pub enum UnityCommand {
    /// Parse a prefab file and show hierarchy
    ParsePrefab {
        /// Path to .prefab file
        prefab_path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List heroes in a Unity project
    ListHeroes {
        /// Path to Unity project (extracted)
        project_path: PathBuf,
    },

    /// Export hero assets
    ExportHero {
        /// Hero ID (e.g., "001", "10280")
        hero_id: String,

        /// Path to Unity project
        project_path: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,
    },
}

pub async fn run(args: UnityArgs) -> anyhow::Result<()> {
    match args.command {
        UnityCommand::ParsePrefab { prefab_path, json } => {
            println!(
                "{} Parsing prefab: {}",
                style("[Unity]").cyan().bold(),
                prefab_path.display()
            );

            let hierarchy = crate::core::unity::prefab::parse_hierarchy(&prefab_path)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&hierarchy)?);
            } else {
                print_hierarchy(&hierarchy, 0);
            }
        }

        UnityCommand::ListHeroes { project_path } => {
            println!(
                "{} Scanning for heroes in: {}",
                style("[Unity]").cyan().bold(),
                project_path.display()
            );

            let heroes = crate::core::unity::scan_heroes(&project_path)?;

            if heroes.is_empty() {
                println!("{} No heroes found", style("[!]").yellow());
            } else {
                println!("{} Found {} heroes:", style("[✓]").green().bold(), heroes.len());
                for hero in &heroes {
                    println!("  - {} ({})", hero.name, hero.id);
                }
            }
        }

        UnityCommand::ExportHero {
            hero_id,
            project_path,
            output,
        } => {
            println!(
                "{} Exporting hero {} from {}",
                style("[Unity]").cyan().bold(),
                hero_id,
                project_path.display()
            );

            crate::core::unity::export_hero(&hero_id, &project_path, &output)?;

            println!(
                "{} Exported to {}",
                style("[✓]").green().bold(),
                output.display()
            );
        }
    }

    Ok(())
}

fn print_hierarchy(node: &crate::core::unity::prefab::HierarchyNode, indent: usize) {
    let prefix = "  ".repeat(indent);

    let icon = if node.children.is_some() && !node.children.as_ref().unwrap().is_empty() {
        "📁"
    } else if node.sprite.is_some() {
        "🖼️"
    } else {
        "📄"
    };

    println!("{}{} {}", prefix, icon, node.name);

    if let Some(children) = &node.children {
        for child in children {
            print_hierarchy(child, indent + 1);
        }
    }
}

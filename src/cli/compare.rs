//! Compare two Unity project versions

use clap::Args;
use console::style;
use std::path::PathBuf;

#[derive(Args)]
pub struct CompareArgs {
    /// Old project directory
    pub old_path: PathBuf,

    /// New project directory
    pub new_path: PathBuf,

    /// Filter by path pattern (e.g., "01_Fx")
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: CompareArgs) -> anyhow::Result<()> {
    use crate::core::comparator;

    println!(
        "{} Comparing Unity projects...",
        style("[Compare]").cyan().bold()
    );
    println!("  Old: {}", args.old_path.display());
    println!("  New: {}", args.new_path.display());

    if let Some(ref filter) = args.filter {
        println!("  Filter: {}", filter);
    }

    let result = comparator::compare_projects(
        &args.old_path,
        &args.new_path,
        args.filter.as_deref(),
    )?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!();
        println!("{}", style("=== Comparison Summary ===").bold());
        println!(
            "  {} Added:     {}",
            style("+").green(),
            result.summary.total_added
        );
        println!(
            "  {} Removed:   {}",
            style("-").red(),
            result.summary.total_removed
        );
        println!(
            "  {} Modified:  {}",
            style("~").yellow(),
            result.summary.total_modified
        );
        println!(
            "  {} Unchanged: {}",
            style("=").dim(),
            result.summary.total_unchanged
        );

        if !result.summary.new_heroes.is_empty() {
            println!();
            println!("{}", style("New Heroes:").green().bold());
            for hero in &result.summary.new_heroes {
                println!("  + {}", hero);
            }
        }

        if !result.summary.removed_heroes.is_empty() {
            println!();
            println!("{}", style("Removed Heroes:").red().bold());
            for hero in &result.summary.removed_heroes {
                println!("  - {}", hero);
            }
        }

        if !result.summary.new_assets.is_empty() {
            println!();
            println!("{}", style("New Assets by Category:").cyan().bold());
            for (category, count) in &result.summary.new_assets {
                println!("  {}: {}", category, count);
            }
        }
    }

    Ok(())
}

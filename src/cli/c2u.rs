//! C2U - Convert XAPK to Unity project

use clap::Args;
use console::style;
use std::path::PathBuf;

#[derive(Args)]
pub struct C2uArgs {
    /// Path to XAPK file
    pub xapk_path: PathBuf,

    /// Output directory for Unity project
    #[arg(short, long)]
    pub output: PathBuf,

    /// Custom tools directory
    #[arg(long, env = "KGC_TOOLS_DIR")]
    pub tools_dir: Option<PathBuf>,
}

pub async fn run(args: C2uArgs) -> anyhow::Result<()> {
    use crate::core::c2u;

    println!(
        "{} Converting XAPK to Unity project...",
        style("[C2U]").cyan().bold()
    );
    println!("  Input:  {}", args.xapk_path.display());
    println!("  Output: {}", args.output.display());

    c2u::convert(
        &args.xapk_path,
        &args.output,
        args.tools_dir.as_deref(),
        |stage, progress, msg| {
            println!(
                "{} [{}%] {}",
                style(format!("[{}]", stage)).cyan(),
                progress,
                msg
            );
        },
    )
    .await?;

    println!(
        "{} Conversion complete!",
        style("[✓]").green().bold()
    );

    Ok(())
}

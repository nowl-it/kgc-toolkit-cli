//! MITM Proxy commands

use clap::{Args, Subcommand};
use console::style;

#[derive(Args)]
pub struct ProxyArgs {
    #[command(subcommand)]
    pub command: ProxyCommand,
}

#[derive(Subcommand)]
pub enum ProxyCommand {
    /// Start the MITM proxy server
    Start {
        /// Port to listen on
        #[arg(short, long, default_value = "8888")]
        port: u16,

        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0")]
        bind: String,
    },

    /// Stop the running proxy server
    Stop,

    /// List captured traffic
    List {
        /// Filter by URL pattern
        #[arg(short, long)]
        filter: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Decrypt a captured response
    Decrypt {
        /// Request ID to decrypt
        request_id: i64,

        /// Output as JSON (pretty-printed)
        #[arg(long)]
        json: bool,
    },

    /// Export captured traffic
    Export {
        /// Output file path
        #[arg(short, long)]
        output: std::path::PathBuf,
    },

    /// Clear all captured traffic
    Clear,

    /// Show proxy status and certificate info
    Status,
}

pub async fn run(args: ProxyArgs) -> anyhow::Result<()> {
    match args.command {
        ProxyCommand::Start { port, bind } => {
            println!(
                "{} Starting MITM proxy on {}:{}...",
                style("[Proxy]").cyan().bold(),
                bind,
                port
            );

            crate::proxy::server::start(port, &bind).await?;
        }

        ProxyCommand::Stop => {
            println!("{} Stopping proxy...", style("[Proxy]").cyan().bold());
            crate::proxy::server::stop().await?;
            println!("{} Proxy stopped", style("[✓]").green().bold());
        }

        ProxyCommand::List { filter, limit, json } => {
            let requests = crate::proxy::storage::get_requests(filter.as_deref(), limit)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&requests)?);
            } else {
                if requests.is_empty() {
                    println!("{} No captured requests", style("[!]").yellow());
                } else {
                    println!("{} Captured requests:", style("[Proxy]").cyan().bold());
                    for req in &requests {
                        println!(
                            "  [{}] {} {} ({})",
                            req.id,
                            style(&req.method).cyan(),
                            req.url,
                            style(req.response_status).yellow()
                        );
                    }
                }
            }
        }

        ProxyCommand::Decrypt { request_id, json } => {
            let decrypted = crate::proxy::storage::decrypt_response(request_id)?;

            if json {
                // Try to parse as JSON and pretty print
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&decrypted) {
                    println!("{}", serde_json::to_string_pretty(&parsed)?);
                } else {
                    println!("{}", decrypted);
                }
            } else {
                println!("{}", decrypted);
            }
        }

        ProxyCommand::Export { output } => {
            let requests = crate::proxy::storage::get_requests(None, usize::MAX)?;
            let json = serde_json::to_string_pretty(&requests)?;
            std::fs::write(&output, json)?;
            println!(
                "{} Exported {} requests to {}",
                style("[✓]").green().bold(),
                requests.len(),
                output.display()
            );
        }

        ProxyCommand::Clear => {
            crate::proxy::storage::clear_all()?;
            println!("{} All traffic cleared", style("[✓]").green().bold());
        }

        ProxyCommand::Status => {
            let status = crate::proxy::server::get_status().await?;

            if let Some(status) = status {
                println!("{}", style("=== Proxy Status ===").bold());
                println!(
                    "  Status:   {}",
                    if status.running {
                        style("Running").green()
                    } else {
                        style("Stopped").red()
                    }
                );
                println!("  Port:     {}", status.port);
                println!("  Local IP: {}", status.local_ip);
                println!("  Cert URL: {}", status.cert_download_url);
            } else {
                println!("{} Proxy is not running", style("[!]").yellow());
            }
        }
    }

    Ok(())
}

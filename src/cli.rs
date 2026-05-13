use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::cmd;

/// CLI for working with Algolia's APIs
#[derive(Parser)]
#[command(name = "Algolia", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Work with indices
    #[command(visible_aliases = ["index", "i"])]
    Indices {
        #[command(subcommand)]
        subcommand: IndicesCommands,
    },
}

#[derive(Subcommand)]
enum IndicesCommands {
    /// List indices
    #[command(visible_aliases = ["ls", "l"])]
    List,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Indices { subcommand } => match subcommand {
            IndicesCommands::List => cmd::list_indices().await?,
        },
    }

    Ok(())
}

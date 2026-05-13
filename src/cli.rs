use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};

use crate::algolia::search;
use crate::auth;
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

    /// Generate completion scripts
    Completions {
        /// The shell for which to generate completions
        #[arg(value_enum)]
        shell: Shell,
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
    let auth = auth::Credentials::from_env()?;
    let client = search::SearchClient::new(&auth.app_id, &auth.api_key)?;

    match cli.command {
        Commands::Indices { subcommand } => match subcommand {
            IndicesCommands::List => cmd::list_indices(&client).await?,
        },
        Commands::Completions { shell } => generate(
            shell,
            &mut Cli::command(),
            "algolia-rs",
            &mut std::io::stdout(),
        ),
    }

    Ok(())
}

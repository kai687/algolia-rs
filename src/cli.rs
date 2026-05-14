use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use clap_complete_nushell::Nushell;
use std::io;
use std::io::IsTerminal;

use crate::algolia::search;
use crate::auth;
use crate::cmd;

/// Handle different output options
#[derive(Clone, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
}

/// `clap_complete::Shell` + Nushell
#[derive(Clone, clap::ValueEnum)]
pub enum AnyShell {
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell", alias = "pwsh")]
    PowerShell,
    Elvish,
    #[value(alias = "nu")]
    Nushell,
}

/// CLI for working with Algolia's APIs
#[derive(clap::Parser)]
#[command(name = "Algolia", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(short, long, global = true)]
    output: Option<OutputFormat>,
}

#[derive(clap::Subcommand)]
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
        shell: AnyShell,
    },
}

#[derive(clap::Subcommand)]
enum IndicesCommands {
    /// List indices
    #[command(visible_aliases = ["ls", "l"])]
    List,

    /// Copy an index
    #[command(visible_aliases = ["cp"])]
    Copy {
        /// Index to copy
        index_name: String,
        /// Destination index
        #[arg(short, long)]
        destination: String,
        /// What to copy
        #[arg(short, long, num_args = 0.., value_delimiter = ',')]
        scope: Vec<search::operation_index::OperationScope>,
    },

    /// Move (rename) an index
    #[command(visible_aliases = ["mv"])]
    Move {
        /// Index to move or rename
        index_name: String,
        /// Destination index
        #[arg(short, long)]
        destination: String,
    },

    /// Delete an index
    #[command(visible_aliases = ["del"])]
    Delete {
        /// Index to delete
        index_name: String,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Set output if not specified by command line option
    let output_format = cli.output.unwrap_or_else(|| {
        if io::stdout().is_terminal() {
            OutputFormat::Pretty
        } else {
            OutputFormat::Json
        }
    });

    match cli.command {
        Commands::Indices { subcommand } => match subcommand {
            IndicesCommands::List => {
                let auth = auth::Credentials::load()?;
                let client = search::SearchClient::new(&auth.app_id, &auth.api_key)?;

                cmd::list_indices(&client, &output_format, io::stdout()).await?;
            }

            IndicesCommands::Copy {
                index_name,
                destination,
                scope,
            } => {
                let auth = auth::Credentials::load()?;
                let client = search::SearchClient::new(&auth.app_id, &auth.api_key)?;

                cmd::copy_index(
                    &client,
                    index_name,
                    &destination,
                    Some(scope),
                    &output_format,
                    io::stdout(),
                )
                .await?;
            }

            IndicesCommands::Move {
                index_name,
                destination,
            } => {
                let auth = auth::Credentials::load()?;
                let client = search::SearchClient::new(&auth.app_id, &auth.api_key)?;

                cmd::move_index(
                    &client,
                    index_name,
                    &destination,
                    &output_format,
                    io::stdout(),
                )
                .await?;
            }

            IndicesCommands::Delete { index_name } => {
                let auth = auth::Credentials::load()?;
                let client = search::SearchClient::new(&auth.app_id, &auth.api_key)?;
                cmd::delete_index(&client, index_name, io::stdout()).await?;
            }
        },

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = env!("CARGO_PKG_NAME");

            // The duplication exists because I can't extend the `clap_complete::Shell` enum.
            match shell {
                AnyShell::Bash => generate(Shell::Bash, &mut cmd, name, &mut io::stdout()),
                AnyShell::Zsh => generate(Shell::Zsh, &mut cmd, name, &mut io::stdout()),
                AnyShell::Fish => generate(Shell::Fish, &mut cmd, name, &mut io::stdout()),
                AnyShell::Elvish => generate(Shell::PowerShell, &mut cmd, name, &mut io::stdout()),
                AnyShell::PowerShell => {
                    generate(Shell::PowerShell, &mut cmd, name, &mut io::stdout());
                }
                AnyShell::Nushell => generate(Nushell, &mut cmd, name, &mut io::stdout()),
            }
        }
    }

    Ok(())
}

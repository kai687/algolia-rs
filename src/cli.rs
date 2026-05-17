use anyhow::Result;
use clap::{Args, CommandFactory, Parser};
use clap_complete::{Shell, generate};
use clap_complete_nushell::Nushell;
use dialoguer::Confirm;
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

/// Options that are valid in many commands
#[derive(Args)]
pub struct GlobalOpts {
    /// Output format
    #[arg(short, long, global = true)]
    pub output: Option<OutputFormat>,

    /// Wait until operations are complete
    #[arg(short, long, global = true)]
    pub wait: bool,

    /// Always confirm the operation
    #[arg(short = 'y', long = "yes", global = true)]
    pub confirm: bool,
}

impl GlobalOpts {
    // Set output if not specified by command line option
    #[must_use]
    pub fn output_format(&self) -> OutputFormat {
        self.output.clone().unwrap_or_else(|| {
            if io::stdout().is_terminal() {
                OutputFormat::Pretty
            } else {
                OutputFormat::Json
            }
        })
    }
}

/// Confirm a potentially destructive operation
#[must_use]
pub fn confirm_operation(msg: &str) -> bool {
    Confirm::new()
        .with_prompt(msg)
        .default(false)
        .interact()
        .unwrap_or(false)
}

/// CLI for working with Algolia's APIs
#[derive(clap::Parser)]
#[command(name = "Algolia", version, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    global: GlobalOpts,
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

/// Helper function to initialize a Search API client
fn require_client() -> Result<search::SearchClient> {
    let auth = auth::Credentials::load()?;
    search::SearchClient::new(&auth.app_id, &auth.api_key)
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Indices { subcommand } => match subcommand {
            IndicesCommands::List => {
                let client = require_client()?;
                cmd::list_indices(&client, &cli.global, io::stdout()).await?;
            }

            IndicesCommands::Copy {
                index_name,
                destination,
                scope,
            } => {
                let client = require_client()?;

                cmd::copy_index(
                    &client,
                    index_name,
                    &destination,
                    Some(scope),
                    &cli.global,
                    io::stdout(),
                )
                .await?;
            }

            IndicesCommands::Move {
                index_name,
                destination,
            } => {
                let client = require_client()?;

                cmd::move_index(&client, index_name, &destination, &cli.global, io::stdout())
                    .await?;
            }

            IndicesCommands::Delete { index_name } => {
                let client = require_client()?;

                cmd::delete_index(&client, index_name, &cli.global, io::stdout()).await?;
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

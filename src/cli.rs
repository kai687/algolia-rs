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
    List {
        /// Output format
        #[arg(short, long, value_enum)]
        output: Option<OutputFormat>,
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Indices { subcommand } => match subcommand {
            IndicesCommands::List { output } => {
                let auth = auth::Credentials::load()?;
                let client = search::SearchClient::new(&auth.app_id, &auth.api_key)?;

                // Set output if not specified by command line option
                let output_format = output.unwrap_or_else(|| {
                    if io::stdout().is_terminal() {
                        OutputFormat::Pretty
                    } else {
                        OutputFormat::Json
                    }
                });
                cmd::list_indices(&client, &output_format, io::stdout()).await?;
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

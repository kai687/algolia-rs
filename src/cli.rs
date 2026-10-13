//! Command-line definition and dispatch.

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::api::SearchClient;
use crate::auth::Credentials;
use crate::cmd;
use crate::github::GitHubActions;
use crate::output::OutputFormat;

/// Exit code when `settings diff` finds drift
const EXIT_DRIFT: u8 = 1;

/// Options that are valid in many commands
#[derive(Args, Default)]
pub struct GlobalOpts {
    /// Output format
    #[arg(short, long, global = true, default_value_t = OutputFormat::Text, value_enum)]
    pub output: OutputFormat,
}

/// Algolia CLI for CI pipelines
///
/// Credentials are read from the environment variables
/// `ALGOLIA_APPLICATION_ID` and `ALGOLIA_API_KEY`.
#[derive(Parser)]
#[command(name = "algolia-rs", version, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    global: GlobalOpts,
}

#[derive(Subcommand)]
enum Commands {
    /// Work with records
    #[command(visible_aliases = ["objects", "r"])]
    Records {
        #[command(subcommand)]
        subcommand: RecordsCommands,
    },

    /// Work with index settings
    #[command(visible_aliases = ["s"])]
    Settings {
        #[command(subcommand)]
        subcommand: SettingsCommands,
    },
}

#[derive(Subcommand)]
enum RecordsCommands {
    /// Replace all records in an index with new ones from a file
    ///
    /// Uploads to a temporary index first, then moves it over the target,
    /// so the target index is never empty. Every record needs an `objectID`.
    Replace {
        /// Index whose records to replace
        index_name: String,

        /// Input file with one JSON record per line (`-` for standard input)
        #[arg(short, long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SettingsCommands {
    /// Print index settings as JSON
    #[command(visible_aliases = ["g"])]
    Get {
        /// Index name
        index_name: String,
    },

    /// Compare a settings file with the settings of an index
    ///
    /// Exit code 0 if they match, 1 if they differ, 2 on error.
    /// In GitHub Actions, drift is also reported as an error annotation
    /// and written to the job summary.
    #[command(visible_aliases = ["d"])]
    Diff {
        /// Index name
        index_name: String,
        /// Settings file (`-` for standard input)
        #[arg(short, long, value_name = "FILE")]
        file: Option<PathBuf>,
    },

    /// Apply a settings file to an index and wait until the settings are live
    #[command(visible_aliases = ["a"])]
    Apply {
        /// Index name
        index_name: String,
        /// Input file (`-` for standard input)
        #[arg(short, long, value_name = "FILE")]
        file: Option<PathBuf>,
    },
}

/// Parse arguments, run the command, return the exit code
pub async fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let creds = Credentials::load()?;
    let client = SearchClient::new(&creds.app_id, &creds.api_key)?;
    let github = GitHubActions::from_env();
    let output = cli.global.output;
    let stdout = io::stdout();

    match cli.command {
        Commands::Records { subcommand } => match subcommand {
            RecordsCommands::Replace { index_name, file } => {
                cmd::replace_records(&client, &index_name, file.as_deref(), stdout).await?;
            }
        },

        Commands::Settings { subcommand } => match subcommand {
            SettingsCommands::Get { index_name } => {
                cmd::get_settings(&client, &index_name, output, stdout).await?;
            }

            SettingsCommands::Diff { index_name, file } => {
                let diff = cmd::diff_settings(
                    &client,
                    &index_name,
                    file.as_deref(),
                    output,
                    &github,
                    stdout,
                )
                .await?;

                if !diff.is_empty() {
                    return Ok(ExitCode::from(EXIT_DRIFT));
                }
            }

            SettingsCommands::Apply { index_name, file } => {
                cmd::apply_settings(&client, &index_name, file.as_deref(), output, stdout).await?;
            }
        },
    }

    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn output_defaults_to_text() {
        let cli = Cli::try_parse_from(["algolia-rs", "settings", "get", "idx"]).unwrap();
        assert_eq!(cli.global.output, OutputFormat::Text);
    }

    #[test]
    fn global_flags_work_after_subcommand() {
        let cli =
            Cli::try_parse_from(["algolia-rs", "settings", "get", "idx", "-o", "json"]).unwrap();
        assert_eq!(cli.global.output, OutputFormat::Json);
    }

    #[test]
    fn replace_accepts_stdin_marker() {
        let cli = Cli::try_parse_from(["algolia-rs", "records", "replace", "idx", "--file", "-"])
            .unwrap();
        match cli.command {
            Commands::Records {
                subcommand: RecordsCommands::Replace { index_name, file },
            } => {
                assert_eq!(index_name, "idx");
                assert_eq!(file, Some(PathBuf::from("-")));
            }
            _ => panic!("expected records replace"),
        }
    }
}

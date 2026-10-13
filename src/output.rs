//! Output format shared by the CLI and the commands.

use clap::ValueEnum;

/// How commands print their results
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable log lines
    #[default]
    Text,
    /// Machine-readable JSON
    Json,
}

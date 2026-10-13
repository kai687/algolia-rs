#![allow(clippy::missing_errors_doc)]

use colored::Colorize;
use std::process::ExitCode;

mod api;
mod auth;
mod cli;
mod cmd;
mod github;
mod output;

/// Exit code for errors (drift uses 1, like `diff`)
const EXIT_ERROR: u8 = 2;

#[tokio::main]
async fn main() -> ExitCode {
    match cli::run().await {
        Ok(code) => code,
        Err(err) => {
            let message = format!("{err:#}");
            if let Some(annotation) = github::GitHubActions::from_env().error_annotation(&message) {
                println!("{annotation}");
            }
            eprintln!("{} {message}", "error:".red().bold());
            ExitCode::from(EXIT_ERROR)
        }
    }
}

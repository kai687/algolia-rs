#![allow(clippy::missing_errors_doc)]

use anyhow::Result;

pub mod algolia;
pub mod auth;
pub mod cli;
pub mod cmd;

#[tokio::main]
async fn main() -> Result<()> {
    cli::run().await
}

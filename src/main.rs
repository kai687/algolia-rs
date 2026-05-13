use anyhow::Result;
use tokio;

pub mod algolia;
pub mod cli;
pub mod cmd;

#[tokio::main]
async fn main() -> Result<()> {
    cli::run().await
}

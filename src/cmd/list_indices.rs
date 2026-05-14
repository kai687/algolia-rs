use crate::algolia::search::SearchClient;
use crate::cli::OutputFormat;
use anyhow::{Context, Result};

/// List all indices
pub async fn list_indices<W: std::io::Write>(
    client: &SearchClient,
    output: &OutputFormat,
    mut writer: W,
) -> Result<()> {
    let response = client
        .list_indices()
        .await
        .context("Listing indices failed")?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&response)
                .context("Failed to convert response back to JSON.")?;

            writeln!(writer, "{json}")?;
        }
        OutputFormat::Pretty => {
            writeln!(writer, "{response}")?;
        }
    }

    Ok(())
}

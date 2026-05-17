use crate::algolia::search::SearchClient;
use crate::cli::{GlobalOpts, OutputFormat};
use anyhow::{Context, Result};

/// List all indices
pub async fn list_indices<W: std::io::Write>(
    client: &SearchClient,
    global_opts: &GlobalOpts,
    mut writer: W,
) -> Result<()> {
    let response = client
        .list_indices()
        .await
        .context("Listing indices failed")?;

    match global_opts.output_format() {
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

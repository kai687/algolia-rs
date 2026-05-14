use crate::algolia::search::SearchClient;
use anyhow::{Context, Result};

/// Delete an index
pub async fn delete_index<W: std::io::Write>(
    client: &SearchClient,
    index_name: String,
    mut writer: W,
) -> Result<()> {
    let response = client
        .delete_index(&index_name)
        .await
        .context("Can't delete index")?;

    let json = serde_json::to_string_pretty(&response).context("Can't convert response to JSON")?;
    writeln!(writer, "{json}")?;

    Ok(())
}

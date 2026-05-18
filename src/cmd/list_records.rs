use anyhow::{Context, Result};

use crate::algolia::search::SearchClient;

/// List all records
pub async fn list_records<W: std::io::Write>(client: &SearchClient, index_name: String, mut writer: W) -> Result<()> {
    let records = client.list_records(&index_name).await?;

    let json = serde_json::to_string_pretty(&records).context("Can't convert response to JSON")?;
    writeln!(writer, "{json}")?;

    Ok(())
}

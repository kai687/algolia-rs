use crate::algolia::search::SearchClient;
use anyhow::{Context, Result};
use serde_json;

/// List all indices
pub async fn list_indices(client: &SearchClient) -> Result<()> {
    let response = client
        .list_indices()
        .await
        .context("Listing indices failed")?;

    let json = serde_json::to_string_pretty(&response)
        .context("Failed to convert response back to JSON.")?;

    println!("{}", json);

    Ok(())
}

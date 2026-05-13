use anyhow::{Context, Result};
use crate::algolia::search::SearchClient;

/// List all indices
pub async fn list_indices(client: &SearchClient) -> Result<()> {
    client.list_indices().await.with_context(|| "Listing indices failed")
}

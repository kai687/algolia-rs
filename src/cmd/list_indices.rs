use anyhow::{Context, Result};
use crate::algolia::search::SearchClient;


pub async fn list_indices() -> Result<()> {
    let app_id = std::env::var("ALGOLIA_APPLICATION_ID")
        .with_context(|| "Missing environment variable: `ALGOLIA_APPLICATION_ID`")?;
    let api_key = std::env::var("ALGOLIA_API_KEY")
        .with_context(|| "Missing environment variable: `ALGOLIA_API_KEY`")?;

    let client = SearchClient::new(&app_id, &api_key)?;

    client.list_indices().await.with_context(|| "Listing indices failed")


}

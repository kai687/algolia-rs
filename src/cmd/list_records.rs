use anyhow::Result;

use crate::algolia::search::SearchClient;

/// List all records
pub async fn list_records(client: &SearchClient, index_name: String) -> Result<()> {
    let records = client.list_records(&index_name).await?;

    for record in records {
        println!("{record:?}");
    }

    Ok(())
}

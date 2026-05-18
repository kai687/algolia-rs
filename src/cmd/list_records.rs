use anyhow::{Context, Result};

use crate::algolia::search::SearchClient;

/// List all records
pub async fn list_records<W: std::io::Write>(
    client: &SearchClient,
    index_name: String,
    mut writer: W,
) -> Result<()> {
    client
        .for_each_record_page(&index_name, |records| {
            // Print each record as its own JSON object as soon as its page arrives.
            for record in records {
                let json =
                    serde_json::to_string_pretty(&record).context("Can't convert response to JSON")?;
                writeln!(writer, "{json}")?;
            }
            writer.flush()?;
            Ok(())
        })
        .await
}

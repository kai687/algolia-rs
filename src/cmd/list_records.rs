use std::io::{BufWriter, Write};

use anyhow::Result;

use crate::algolia::search::SearchClient;

/// List all records
pub async fn list_records<W: Write>(
    client: &SearchClient,
    index_name: String,
    writer: W,
) -> Result<()> {
    let mut writer = BufWriter::new(writer);

    client
        .list_records(&index_name, |records| {
            for record in records {
                writeln!(writer, "{}", record.get())?;
            }
            Ok(())
        })
        .await?;

    writer.flush()?;
    Ok(())
}

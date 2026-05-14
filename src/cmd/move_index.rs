use crate::algolia::search::{
    SearchClient,
    operation_index::MoveOperation,
};
use anyhow::{Context, Result};

/// Copy an index
pub async fn move_index<W: std::io::Write>(
    client: &SearchClient,
    index_name: String,
    destination: &str,
    mut writer: W,
) -> Result<()> {
    let data = MoveOperation {
        destination: destination.to_string(),
    };

    let response = client
        .move_index(&index_name, &data)
        .await
        .context("Can't move index")?;

    let json = serde_json::to_string_pretty(&response).context("Can't convert response to JSON")?;
    writeln!(writer, "{json}")?;

    Ok(())
}

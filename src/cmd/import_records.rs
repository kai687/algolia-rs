use crate::algolia::search::SearchClient;
use anyhow::Result;
use std::path::PathBuf;

pub async fn import_records<W: std::io::Write>(
    _client: &SearchClient,
    index_name: String,
    _file: Option<PathBuf>,
    _auto_id: bool,
    mut writer: W,
) -> Result<()> {
    writeln!(writer, "Indexing records into {index_name}")?;
    Ok(())
}

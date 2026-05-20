use crate::{algolia::search::SearchClient, cmd::file_or_stdin};
use anyhow::Result;
use std::path::PathBuf;

pub async fn import_settings<W: std::io::Write>(
    client: &SearchClient,
    index_name: &str,
    file: Option<PathBuf>,
    mut writer: W,
) -> Result<()> {
    let input = file_or_stdin(file.as_ref())?;
    let settings: serde_json::Value = serde_json::from_reader(input)?;
    client.set_settings(index_name, &settings).await?;

    writeln!(writer, "Updated index settings!")?;

    Ok(())
}

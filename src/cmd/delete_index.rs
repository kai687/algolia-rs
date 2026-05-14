use crate::{algolia::search::SearchClient, cli::OutputFormat};
use anyhow::{Context, Result};
use colored::Colorize;

/// Delete an index
pub async fn delete_index<W: std::io::Write>(
    client: &SearchClient,
    index_name: String,
    output: &OutputFormat,
    wait: bool,
    mut writer: W,
) -> Result<()> {
    let response = client
        .delete_index(&index_name)
        .await
        .context("Can't delete index")?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&response)
                .context("Can't convert response to JSON")?;
            writeln!(writer, "{json}")?;
        }

        OutputFormat::Pretty => {
            writeln!(writer, "{response}")?;
        }
    }

    if wait {
        client.wait_for_task(&index_name, response.task_id).await?;
        writeln!(
            writer,
            "{} Deleted index {}",
            "🗸".green(),
            index_name.bold(),
        )?;
    }

    Ok(())
}

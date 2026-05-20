use crate::{
    algolia::search::SearchClient,
    cli::{GlobalOpts, OutputFormat, confirm_operation},
};
use anyhow::{Context, Result};
use colored::Colorize;

/// Delete an index
pub async fn delete_index<W: std::io::Write>(
    client: &SearchClient,
    index_name: &str,
    global_opts: &GlobalOpts,
    mut writer: W,
) -> Result<()> {
    if !global_opts.confirm && !confirm_operation("Do you really want to delete this index?") {
        println!("Cancelled");
        return Ok(());
    }

    let response = client
        .delete_index(index_name)
        .await
        .context("Can't delete index")?;

    match global_opts.output_format() {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&response)
                .context("Can't convert response to JSON")?;
            writeln!(writer, "{json}")?;
        }

        OutputFormat::Pretty => {
            writeln!(writer, "{response}")?;
        }
    }

    if global_opts.wait {
        client.wait_for_task(index_name, response.task_id).await?;
        writeln!(
            writer,
            "{} Deleted index {}",
            "🗸".green(),
            index_name.bold(),
        )?;
    }

    Ok(())
}

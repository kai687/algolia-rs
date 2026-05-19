use crate::{
    algolia::search::{SearchClient, operation_index::MoveOperation},
    cli::{GlobalOpts, OutputFormat, confirm_operation},
};
use anyhow::{Context, Result};
use colored::Colorize;

/// Copy an index
pub async fn move_index<W: std::io::Write>(
    client: &SearchClient,
    index_name: String,
    destination: &str,
    global_opts: &GlobalOpts,
    mut writer: W,
) -> Result<()> {
    if !global_opts.confirm
        && client.index_exists(destination).await?
        && !confirm_operation("Destination index exists. Overwrite it?")
    {
        println!("Cancelled");
        return Ok(());
    }

    let data = MoveOperation {
        destination: destination.to_string(),
    };

    let response = client
        .move_index(&index_name, &data)
        .await
        .context("Can't move index")?;

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
        client.wait_for_task(destination, response.task_id).await?;
        writeln!(
            writer,
            "{} Moved index {} to {}",
            "🗸".green(),
            index_name.bold(),
            destination.bold()
        )?;
    }

    Ok(())
}

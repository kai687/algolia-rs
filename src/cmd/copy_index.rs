use crate::{
    algolia::search::{
        SearchClient,
        operation_index::{CopyOperation, OperationScope},
    },
    cli::{GlobalOpts, OutputFormat, confirm_operation},
};
use anyhow::{Context, Result};
use colored::Colorize;

/// Copy an index
pub async fn copy_index<W: std::io::Write>(
    client: &SearchClient,
    index_name: String,
    destination: &str,
    scope: Option<Vec<OperationScope>>,
    global_opts: &GlobalOpts,
    mut writer: W,
) -> Result<()> {
    // Protect against accidentally overwriting destination index
    // unless pre-confirmed
    if !global_opts.confirm
        && client.index_exists(destination).await?
        && !confirm_operation("Destination index exist. Overwrite it?")
    {
        println!("Cancelled.");
        return Ok(());
    }

    let data = CopyOperation {
        destination: destination.to_string(),
        scope,
    };

    let response = client
        .copy_index(&index_name, data)
        .await
        .context("Can't copy index")?;

    match global_opts.output_format() {
        OutputFormat::Json => {
            let json = serde_json::to_string(&response)
                .context("Can't convert Copy index response to JSON")?;
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
            "{} Copied index {} to {}",
            "🗸".green(),
            index_name.bold(),
            destination.bold()
        )?;
    }

    Ok(())
}

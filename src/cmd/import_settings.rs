use crate::{
    algolia::search::SearchClient,
    cli::{GlobalOpts, OutputFormat},
    cmd::file_or_stdin,
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

pub async fn import_settings<W: std::io::Write>(
    client: &SearchClient,
    index_name: &str,
    file: Option<PathBuf>,
    global_opts: &GlobalOpts,
    mut writer: W,
) -> Result<()> {
    let input = file_or_stdin(file.as_ref())?;
    let settings: serde_json::Value = serde_json::from_reader(input)?;
    let response = client.set_settings(index_name, &settings).await?;

    match global_opts.output_format() {
        OutputFormat::Json => {
            let json = serde_json::to_string(&response)
                .context("Can't convert get settings response to JSON")?;
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
            "{} Updated settings of {}",
            "🗸".green(),
            index_name.bold()
        )?;
    }

    Ok(())
}

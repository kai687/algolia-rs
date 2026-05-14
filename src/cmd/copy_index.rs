use crate::{
    algolia::search::{
        SearchClient,
        operation_index::{CopyOperation, OperationScope},
    },
    cli::OutputFormat,
};
use anyhow::{Context, Result};

/// Copy an index
pub async fn copy_index<W: std::io::Write>(
    client: &SearchClient,
    index_name: String,
    destination: &str,
    scope: Option<Vec<OperationScope>>,
    output: &OutputFormat,
    mut writer: W,
) -> Result<()> {
    let data = CopyOperation {
        destination: destination.to_string(),
        scope,
    };

    let response = client
        .copy_index(&index_name, &data)
        .await
        .context("Can't copy index")?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string(&response)
                .context("Can't convert Copy index response to JSON")?;
            writeln!(writer, "{json}")?;
        }
        OutputFormat::Pretty => {
            writeln!(writer, "{response}")?;
        }
    }

    Ok(())
}

use crate::{
    algolia::search::SearchClient,
    cli::{GlobalOpts, OutputFormat},
};
use anyhow::{Context, Result};

/// List index settings
pub async fn get_settings<W: std::io::Write>(
    client: &SearchClient,
    index_name: &str,
    global_opts: &GlobalOpts,
    mut writer: W,
) -> Result<()> {
    let settings = client.get_settings(index_name).await?;

    match global_opts.output_format() {
        OutputFormat::Json => writeln!(writer, "{settings}")?,
        OutputFormat::Pretty => {
            let json = serde_json::to_string_pretty(&settings)
                .context("Can't convert settings to pretty JSON string")?;
            writeln!(writer, "{json}")?;
        }
    }

    Ok(())
}

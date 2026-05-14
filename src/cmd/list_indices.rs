use crate::algolia::search::{SearchClient, list_indices::ListIndexItem};
use crate::cli::OutputFormat;
use anyhow::{Context, Result};
use serde_json;
use tabled::{
    Table,
    settings::{
        Alignment, Color, Style,
        object::{Columns, Rows},
        style::HorizontalLine,
    },
};

/// List all indices
pub async fn list_indices<W: std::io::Write>(
    client: &SearchClient,
    output: &OutputFormat,
    mut writer: W,
) -> Result<()> {
    let response = client
        .list_indices()
        .await
        .context("Listing indices failed")?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&response)
                .context("Failed to convert response back to JSON.")?;

            writeln!(writer, "{json}")?;
        }
        OutputFormat::Pretty => {
            let indices: Vec<ListIndexItem> = response
                .items
                .into_iter()
                .map(ListIndexItem::with_index_type)
                .collect();

            let style = Style::modern_rounded()
                .horizontals([(1, HorizontalLine::inherit(Style::modern_rounded()))])
                .remove_vertical()
                .remove_horizontal()
                .remove_frame();

            let mut table = Table::new(&indices);
            table.with(style);
            table.modify(Rows::first(), Color::FG_BLUE);
            // Records column
            table.modify(Columns::one(1), Alignment::right());
            // Size column
            table.modify(Columns::one(2), Alignment::right());
            writeln!(writer, "{table}")?;
        }
    }

    Ok(())
}

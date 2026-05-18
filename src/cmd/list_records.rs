use std::io::{BufWriter, Write};

use anyhow::{Context, Result};

use crate::{
    algolia::search::SearchClient,
    cli::{GlobalOpts, OutputFormat},
};

/// List all records
pub async fn list_records<W: Write>(
    client: &SearchClient,
    index_name: String,
    global_opts: &GlobalOpts,
    writer: W,
) -> Result<()> {
    let mut writer = BufWriter::new(writer);
    let output_format = global_opts.output_format();

    match output_format {
        OutputFormat::Json => {
            client
                .for_each_raw_record_page(&index_name, |records| {
                    // Print raw record JSON as soon as its page arrives.
                    for record in records {
                        writeln!(writer, "{}", record.get())?;
                    }
                    Ok(())
                })
                .await?;
        }
        OutputFormat::Pretty => {
            client
                .for_each_record_page(&index_name, |records| {
                    // Print each record as its own pretty JSON object as soon as its page arrives.
                    for record in records {
                        serde_json::to_writer_pretty(&mut writer, &record)
                            .context("Can't convert response to JSON")?;
                        writeln!(writer)?;
                    }
                    Ok(())
                })
                .await?;
        }
    }

    writer.flush()?;
    Ok(())
}

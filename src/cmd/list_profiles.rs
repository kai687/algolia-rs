use anyhow::Result;
use crate::cli::OutputFormat;

pub fn list_profiles<W: std::io::Write>(_: &OutputFormat, mut writer: W) -> Result<()> {
    writeln!(writer, "Listing profiles...")?;
    Ok(())
}

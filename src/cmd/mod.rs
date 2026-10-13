//! Commands: orchestrate API calls and format output.
//!
//! Every command takes a writer so tests can capture output.

use anyhow::{Context, Result};
use std::{fs, io, path::Path};

mod records_replace;
mod settings_apply;
mod settings_diff;
mod settings_get;

pub use records_replace::replace_records;
pub use settings_apply::apply_settings;
pub use settings_diff::diff_settings;
pub use settings_get::get_settings;

/// Open `path` for reading, or standard input for `None` or `-`
fn file_or_stdin(path: Option<&Path>) -> Result<Box<dyn io::Read>> {
    match path {
        None => Ok(Box::new(io::stdin())),
        Some(p) if p.as_os_str() == "-" => Ok(Box::new(io::stdin())),
        Some(p) => {
            let file =
                fs::File::open(p).with_context(|| format!("Can't open file {}", p.display()))?;
            Ok(Box::new(file))
        }
    }
}

/// Human-readable name of the input for log lines
fn input_name(path: Option<&Path>) -> String {
    match path {
        None => "stdin".to_string(),
        Some(p) if p.as_os_str() == "-" => "stdin".to_string(),
        Some(p) => p.display().to_string(),
    }
}

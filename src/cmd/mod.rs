use anyhow::Result;
use std::{fs, io, path};

pub mod list_indices;

pub use list_indices::list_indices;

pub mod copy_index;
pub use copy_index::copy_index;

pub mod move_index;
pub use move_index::move_index;

pub mod delete_index;
pub use delete_index::delete_index;

pub mod import_records;
pub use import_records::import_records;

pub mod list_records;
pub use list_records::list_records;

pub mod replace_records;
pub use replace_records::replace_records;

pub mod list_settings;
pub use list_settings::list_settings;

pub mod import_settings;
pub use import_settings::import_settings;

fn file_or_stdin(path: Option<&path::PathBuf>) -> Result<Box<dyn io::Read>> {
    match path.and_then(|p| p.to_str()) {
        None | Some("-") => Ok(Box::new(io::stdin())),
        Some(p) => Ok(Box::new(fs::File::open(p)?)),
    }
}

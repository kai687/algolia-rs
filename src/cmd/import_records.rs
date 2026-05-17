use crate::algolia::search::{
    Record, RecordWithObjectID, SearchClient,
    push_records::{BatchRequestItem, IntoBatchRequestItem},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

fn file_or_stdin(path: Option<&PathBuf>) -> Result<Box<dyn std::io::Read>> {
    match path.and_then(|p| p.to_str()) {
        None | Some("-") => Ok(Box::new(io::stdin())),
        Some(p) => Ok(Box::new(fs::File::open(p)?)),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum AnyRecord {
    RecordWithObjectID(RecordWithObjectID),
    Record(Record),
}

impl IntoBatchRequestItem for AnyRecord {
    fn into_batch_request_item(self) -> BatchRequestItem {
        match self {
            AnyRecord::RecordWithObjectID(r) => r.into_batch_request_item(),
            AnyRecord::Record(r) => r.into_batch_request_item(),
        }
    }
}

pub async fn import_records<W: std::io::Write>(
    client: &SearchClient,
    index_name: String,
    file: Option<PathBuf>,
    mut writer: W,
) -> Result<()> {
    let input = file_or_stdin(file.as_ref())?;
    let mut records_to_push: Vec<Record> = vec![];
    let mut records_to_upsert: Vec<RecordWithObjectID> = vec![];

    for result in serde_json::Deserializer::from_reader(input).into_iter::<AnyRecord>() {
        match result? {
            AnyRecord::RecordWithObjectID(r) => records_to_upsert.push(r),
            AnyRecord::Record(r) => records_to_push.push(r),
        }
    }

    if !records_to_upsert.is_empty() {
        let task_ids = client
            .upsert_records(&index_name, records_to_upsert.into_iter().map(Ok))
            .await?;
        writeln!(
            writer,
            "Replaced {} batches of records in {}",
            task_ids.len(),
            index_name
        )?;
    }

    if !records_to_push.is_empty() {
        let task_ids = client
            .push_records(&index_name, records_to_push.into_iter().map(Ok))
            .await?;
        writeln!(
            writer,
            "Added {} batches of records to {}",
            task_ids.len(),
            index_name
        )?;
    }

    Ok(())
}

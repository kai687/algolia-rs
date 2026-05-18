use crate::algolia::search::{
    RecordWithObjectID, SearchClient,
    operation_index::{CopyOperation, MoveOperation, OperationScope},
};
use anyhow::Result;
use std::path::PathBuf;

pub async fn replace_records<W: std::io::Write>(
    client: &SearchClient,
    index_name: String,
    file: Option<PathBuf>,
    mut writer: W,
) -> Result<()> {
    let input = super::file_or_stdin(file.as_ref())?;
    let tmp_index = format!("{index_name}-tmp");

    // Step 1. Copy index (settings, rules, synonyms) to temp index
    let op = CopyOperation {
        destination: tmp_index.clone(),
        scope: Some(vec![
            OperationScope::Settings,
            OperationScope::Rules,
            OperationScope::Synonyms,
        ]),
    };

    let copy_response = client.copy_index(&index_name, &op).await?;
    client
        .wait_for_task(&tmp_index, copy_response.task_id)
        .await?;

    // Step 2. Add records from file to temp index
    let records = serde_json::Deserializer::from_reader(input)
        .into_iter::<RecordWithObjectID>()
        .map(|record| record.map_err(Into::into));

    let upsert_task_ids = client
        .upsert_records(&tmp_index, records.into_iter())
        .await?;

    for task_id in upsert_task_ids {
        client.wait_for_task(&tmp_index, task_id).await?;
    }

    // Step 3. Move temp index to main index, overwriting it
    let move_op = MoveOperation {
        destination: index_name.clone(),
    };

    let move_response_result = client.move_index(&tmp_index, &move_op).await;

    match move_response_result {
        Ok(response) => client.wait_for_task(&index_name, response.task_id).await?,
        Err(err) => {
            // Clean up temporary index (don't need to wait)
            let _ = client.delete_index(&tmp_index).await?;
            return Err(err);
        }
    }

    writeln!(writer, "Replaced records in {index_name}")?;

    Ok(())
}

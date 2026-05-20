use crate::algolia::search::{
    RecordWithObjectID, SearchClient,
    operation_index::{CopyOperation, MoveOperation, OperationScope},
};
use anyhow::Result;
use std::path::PathBuf;

pub async fn replace_records<W: std::io::Write>(
    client: &SearchClient,
    index_name: &str,
    file: Option<PathBuf>,
    mut writer: W,
) -> Result<()> {
    let input = super::file_or_stdin(file.as_ref())?;
    let tmp_index = format!("{index_name}-tmp");
    let mut tmp_index_created = false;

    let result = async {
        // Step 1. Copy index (settings, rules, synonyms) to temp index
        let op = CopyOperation {
            destination: tmp_index.clone(),
            scope: Some(vec![
                OperationScope::Settings,
                OperationScope::Rules,
                OperationScope::Synonyms,
            ]),
        };

        let copy_response = client.copy_index(index_name, op).await?;
        tmp_index_created = true;

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
            destination: index_name.to_string(),
        };

        let move_response = client.move_index(&tmp_index, move_op).await?;
        client
            .wait_for_task(index_name, move_response.task_id)
            .await?;

        writeln!(writer, "Replaced records in {index_name}")?;

        Ok(())
    }
    .await;

    // Ensure cleanup of temporary index
    if result.is_err() && tmp_index_created {
        let _ = client.delete_index(&tmp_index).await;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algolia::search::test_support::AlgoliaMock;
    use httpmock::Method::{DELETE, GET, POST};
    use serde_json::json;
    use std::io::Write;

    fn record_file() -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, r#"{{"objectID":"1","name":"foo"}}"#).unwrap();
        file
    }

    #[tokio::test]
    async fn replace_records_succeeds() {
        let api = AlgoliaMock::new();

        let copy = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/operation")
                .json_body(json!({
                    "operation": "copy",
                    "destination": "test-index-tmp",
                    "scope": ["settings", "rules", "synonyms"]
                }));

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "taskID": 1,
                    "updatedAt": "2026-05-19T08:50:12.848Z"
                }));
        });

        let wait_copy = api.json_response(
            GET,
            "/1/indexes/test-index-tmp/task/1",
            200,
            json!({"status": "published"}),
        );

        let upload = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index-tmp/batch")
                .json_body(json!({
                    "requests": [
                    {
                        "action": "updateObject",
                        "body": {
                            "objectID": "1",
                            "name": "foo"
                        }
                    }
                    ]
                }));

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "objectIDs": ["1"],
                    "taskID": 2,
                }));
        });

        let wait_upload = api.json_response(
            GET,
            "/1/indexes/test-index-tmp/task/2",
            200,
            json!({"status": "published"}),
        );

        let move_index = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index-tmp/operation")
                .json_body(json!({
                    "operation": "move",
                    "destination": "test-index"
                }));

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "taskID": 3,
                    "updatedAt": "2026-05-19T08:50:12.848Z"
                }));
        });

        let wait_move = api.json_response(
            GET,
            "/1/indexes/test-index/task/3",
            200,
            json!({"status": "published"}),
        );

        let file = record_file();
        let mut output = Vec::new();

        replace_records(
            &api.client,
            "test-index",
            Some(file.path().to_path_buf()),
            &mut output,
        )
        .await
        .unwrap();

        copy.assert();
        wait_copy.assert();
        upload.assert();
        wait_upload.assert();
        move_index.assert();
        wait_move.assert();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Replaced records in test-index\n"
        );
    }

    #[tokio::test]
    async fn replace_records_deletes_tmp_index_when_move_fails() {
        let api = AlgoliaMock::new();

        let copy = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/operation")
                .json_body(json!({
                    "operation": "copy",
                    "destination": "test-index-tmp",
                    "scope": ["settings", "rules", "synonyms"]
                }));

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "taskID": 1,
                    "updatedAt": "2026-05-19T08:50:12.848Z"
                }));
        });

        let wait_copy = api.json_response(
            GET,
            "/1/indexes/test-index-tmp/task/1",
            200,
            json!({"status": "published"}),
        );

        let upload = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index-tmp/batch")
                .json_body(json!({
                    "requests": [
                    {
                        "action": "updateObject",
                        "body": {
                            "objectID": "1",
                            "name": "foo"
                        }
                    }
                    ]
                }));

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "objectIDs": ["1"],
                    "taskID": 2,
                }));
        });

        let wait_upload = api.json_response(
            GET,
            "/1/indexes/test-index-tmp/task/2",
            200,
            json!({"status": "published"}),
        );

        let move_index = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index-tmp/operation")
                .json_body(json!({
                    "operation": "move",
                    "destination": "test-index"
                }));

            then.status(500)
                .header("content-type", "application/json")
                .json_body(json!({
                    "message": "move failed",
                    "status": 500
                }));
        });

        let delete_tmp = api.json_response(
            DELETE,
            "/1/indexes/test-index-tmp",
            200,
            json!({"taskID": 4, "deletedAt": "2026-05-19T08:50:12.848Z"}),
        );

        let file = record_file();
        let mut output = Vec::new();

        let result = replace_records(
            &api.client,
            "test-index",
            Some(file.path().to_path_buf()),
            &mut output,
        )
        .await;

        assert!(result.is_err());

        copy.assert();
        wait_copy.assert();
        upload.assert();
        wait_upload.assert();
        move_index.assert();
        delete_tmp.assert();
    }
}

use crate::{
    api::{SearchClient, indices::Scope, records::RecordWithObjectID},
    cmd::file_or_stdin,
};
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

/// Replace all records of an index.
///
/// 1. Copy settings, rules, and synonyms to a temporary index.
/// 2. Upload the records there.
/// 3. Move the temporary index over the target.
///
/// The target is never empty. On failure, the temporary index is deleted.
pub async fn replace_records<W: std::io::Write>(
    client: &SearchClient,
    index_name: &str,
    file: Option<&Path>,
    mut writer: W,
) -> Result<()> {
    let input = file_or_stdin(file)?;
    let tmp_index = format!("{index_name}-tmp");
    let mut tmp_index_created = false;

    let result = async {
        // Step 1. Copy index (settings, rules, synonyms) to temp index
        let copy_response = client
            .copy_index(
                index_name,
                &tmp_index,
                Some(&[Scope::Settings, Scope::Rules, Scope::Synonyms]),
            )
            .await?;
        tmp_index_created = true;

        client
            .wait_for_task(&tmp_index, copy_response.task_id)
            .await?;
        writeln!(
            writer,
            "{} Copied settings, rules, and synonyms to {}",
            "🗸".green(),
            tmp_index.bold()
        )?;

        // Step 2. Add records from file to temp index
        let mut record_count: usize = 0;
        let records = serde_json::Deserializer::from_reader(input)
            .into_iter::<RecordWithObjectID>()
            .map(|record| {
                record_count += 1;
                record.map_err(Into::into)
            });

        let upsert_task_ids = client.upsert_records(&tmp_index, records).await?;
        let batch_count = upsert_task_ids.len();

        for task_id in upsert_task_ids {
            client.wait_for_task(&tmp_index, task_id).await?;
        }
        writeln!(
            writer,
            "{} Uploaded {record_count} records in {batch_count} batches to {}",
            "🗸".green(),
            tmp_index.bold()
        )?;

        // Step 3. Move temp index to main index, overwriting it
        let move_response = client.move_index(&tmp_index, index_name).await?;
        client
            .wait_for_task(index_name, move_response.task_id)
            .await?;

        writeln!(
            writer,
            "{} Replaced records in {}",
            "🗸".green(),
            index_name.bold()
        )?;

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
    use crate::api::test_support::AlgoliaMock;
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

        replace_records(&api.client, "test-index", Some(file.path()), &mut output)
            .await
            .unwrap();

        copy.assert();
        wait_copy.assert();
        upload.assert();
        wait_upload.assert();
        move_index.assert();
        wait_move.assert();

        let output = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Copied settings, rules, and synonyms to test-index-tmp"));
        assert!(lines[1].contains("Uploaded 1 records in 1 batches to test-index-tmp"));
        assert!(lines[2].contains("Replaced records in test-index"));
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

        let result =
            replace_records(&api.client, "test-index", Some(file.path()), &mut output).await;

        assert!(result.is_err());

        copy.assert();
        wait_copy.assert();
        upload.assert();
        wait_upload.assert();
        move_index.assert();
        delete_tmp.assert();
    }
}

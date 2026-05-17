use super::{Record, RecordWithObjectID, SearchClient};
use anyhow::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait IntoBatchRequestItem {
    fn into_batch_request_item(self) -> BatchRequestItem;
}

impl IntoBatchRequestItem for Record {
    // `Record` -> `BatchRequestItem`
    fn into_batch_request_item(self) -> BatchRequestItem {
        BatchRequestItem {
            action: BatchAction::AddObject,
            body: Value::Object(self.0),
        }
    }
}

impl IntoBatchRequestItem for RecordWithObjectID {
    // `RecordWithObjectID` -> `BatchRequestItem`
    fn into_batch_request_item(self) -> BatchRequestItem {
        BatchRequestItem {
            action: BatchAction::UpdateObject,
            body: serde_json::to_value(self).unwrap(),
        }
    }
}

#[derive(Serialize)]
pub struct BatchRequest {
    requests: Vec<BatchRequestItem>,
}

impl BatchRequest {
    fn new<R: IntoBatchRequestItem>(records: Vec<R>) -> Self {
        BatchRequest {
            requests: records
                .into_iter()
                .map(IntoBatchRequestItem::into_batch_request_item)
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct BatchRequestItem {
    action: BatchAction,
    body: Value,
}

/// Response of a Batch request
/// We don't make use of the `objectIDs` field.
#[derive(Deserialize)]
pub struct BatchResponse {
    /// Task ID
    #[serde(rename = "taskID")]
    task_id: u64,
}

/// Which action to perform (I only need `Add`)
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum BatchAction {
    /// Add a new record (with auto-generated `objectID`)
    AddObject,
    /// Update existing record or add a new one (requires `objectID`)
    UpdateObject,
}

impl SearchClient {
    /// Function for making a batch request to a single index
    async fn batch<R: IntoBatchRequestItem>(
        &self,
        index_name: &str,
        records: Vec<R>,
    ) -> Result<BatchResponse> {
        let request_url = format!("/indexes/{index_name}/batch");
        let request = BatchRequest::new(records);
        self.post(&request_url, Some(&request)).await
    }

    /// Helper function to make a chunked batch request
    /// Returns a list of task IDs
    async fn chunked_batch<R: IntoBatchRequestItem>(
        &self,
        index_name: &str,
        records: impl Iterator<Item = Result<R>>,
    ) -> Result<Vec<u64>> {
        let mut task_ids = Vec::new();
        let chunk_size = 1000;
        for chunk in &records.chunks(chunk_size) {
            let batch = chunk.collect::<Result<Vec<_>, _>>()?;
            let response = self.batch(index_name, batch).await?;
            task_ids.push(response.task_id);
        }

        Ok(task_ids)
    }

    /// Push new records to an Algolia index
    /// Records will be added with an auto-generated `objectID`
    pub async fn push_records(
        &self,
        index_name: &str,
        records: impl Iterator<Item = Result<Record>>,
    ) -> Result<Vec<u64>> {
        self.chunked_batch(index_name, records).await
    }

    /// Upsert new records to an Algolia index
    /// Records with matching `objectID` will be replaced.
    pub async fn upsert_records(
        &self,
        index_name: &str,
        records: impl Iterator<Item = Result<RecordWithObjectID>>,
    ) -> Result<Vec<u64>> {
        self.chunked_batch(index_name, records).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use serde_json::json;

    #[test]
    fn single_batch_request_item_with_auto_id_sets_action() {
        let test_json = json!({"name": "test-record", "prop": "test-prop"});
        let test_record: Record = serde_json::from_value(test_json).unwrap();
        let req = test_record.into_batch_request_item();

        assert_eq!(req.action, BatchAction::AddObject);
        assert_eq!(req.body.get("name").unwrap(), "test-record");
        assert_eq!(req.body.get("prop").unwrap(), "test-prop");
    }

    #[test]
    fn single_batch_request_item_without_auto_id_sets_action() {
        let test_json = json!({"objectID": "1", "name": "test-record", "prop": "test-prop"});
        let test_record: RecordWithObjectID = serde_json::from_value(test_json).unwrap();
        let req = test_record.into_batch_request_item();
        assert_eq!(req.action, BatchAction::UpdateObject);
        assert_eq!(req.body.get("objectID").unwrap(), "1");
    }

    #[test]
    fn single_batch_request_item_without_auto_id_type_error() {
        let test_json = json!({"name": "test-record", "prop": "test-prop"});
        let test_record = serde_json::from_value::<RecordWithObjectID>(test_json);
        assert!(test_record.is_err());
    }

    #[test]
    fn batch_request_constructs_from_with_auto_id() {
        let test_json = json!([{
            "name": "foo",
            "prop": "foo-prop",
        }, {
            "name": "bar",
            "prop": "bar-prop",
        }]);
        let test_records: Vec<Record> = serde_json::from_value(test_json).unwrap();
        let batch_req = BatchRequest::new(test_records);

        assert_eq!(batch_req.requests.len(), 2);
    }

    #[tokio::test]
    async fn push_records_sends_successfully() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/1/indexes/test-index/batch");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"objectIDs": ["1", "2", "3"], "taskID": 123_456}));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let test_json = vec![
            json!({
                "name": "foo",
                "prop": "foo-prop",
            }),
            json!({
                "name": "bar",
                "prop": "bar-prop",
            }),
        ];

        let test_records = test_json
            .into_iter()
            .map(|v| serde_json::from_value::<Record>(v).map_err(Into::into));

        client
            .push_records("test-index", test_records)
            .await
            .unwrap();
        mock.assert();
    }

    #[tokio::test]
    async fn upsert_records_sends_successfully() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/1/indexes/test-index/batch");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"objectIDs": ["1", "2", "3"], "taskID": 123_456}));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let test_json = vec![
            json!({
                "objectID": "1",
                "name": "foo",
                "prop": "foo-prop",
            }),
            json!({
                "objectID": "2",
                "name": "bar",
                "prop": "bar-prop",
            }),
        ];
        let test_records = test_json
            .into_iter()
            .map(|v| serde_json::from_value::<RecordWithObjectID>(v).map_err(Into::into));

        client
            .upsert_records("test-index", test_records)
            .await
            .unwrap();
        mock.assert();
    }
}

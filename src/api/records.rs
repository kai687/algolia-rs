use super::SearchClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Algolia record. Every record needs an `objectID`.
#[derive(Debug, Serialize, Deserialize)]
pub struct RecordWithObjectID {
    /// A unique identifier for the record
    #[serde(rename = "objectID")]
    pub object_id: String,

    /// Record attributes
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

/// Records per batch request
const CHUNK_SIZE: usize = 1000;

#[derive(Serialize)]
pub struct BatchRequest {
    requests: Vec<BatchRequestItem>,
}

impl BatchRequest {
    fn new(records: Vec<RecordWithObjectID>) -> Self {
        BatchRequest {
            requests: records.into_iter().map(BatchRequestItem::from).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct BatchRequestItem {
    action: BatchAction,
    body: Value,
}

impl From<RecordWithObjectID> for BatchRequestItem {
    fn from(record: RecordWithObjectID) -> Self {
        BatchRequestItem {
            action: BatchAction::UpdateObject,
            body: serde_json::to_value(record).expect("record is valid JSON"),
        }
    }
}

/// Response of a Batch request
/// We don't make use of the `objectIDs` field.
#[derive(Deserialize)]
pub struct BatchResponse {
    /// Task ID
    #[serde(rename = "taskID")]
    task_id: u64,
}

/// Which action to perform
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum BatchAction {
    /// Update existing record or add a new one (requires `objectID`)
    UpdateObject,
}

impl SearchClient {
    /// Function for making a batch request to a single index
    async fn batch(
        &self,
        index_name: &str,
        records: Vec<RecordWithObjectID>,
    ) -> Result<BatchResponse> {
        let request_url = format!("/indexes/{index_name}/batch");
        let request = BatchRequest::new(records);
        self.post(&request_url, &request).await
    }

    /// Upsert records into an Algolia index in chunks of [`CHUNK_SIZE`].
    /// Records with matching `objectID` will be replaced.
    /// Returns one task ID per batch request.
    pub async fn upsert_records(
        &self,
        index_name: &str,
        records: impl Iterator<Item = Result<RecordWithObjectID>>,
    ) -> Result<Vec<u64>> {
        let mut task_ids = Vec::new();
        let mut chunk = Vec::with_capacity(CHUNK_SIZE);

        for record in records {
            chunk.push(record?);
            if chunk.len() == CHUNK_SIZE {
                let response = self.batch(index_name, std::mem::take(&mut chunk)).await?;
                task_ids.push(response.task_id);
            }
        }

        if !chunk.is_empty() {
            let response = self.batch(index_name, chunk).await?;
            task_ids.push(response.task_id);
        }

        Ok(task_ids)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::test_support::AlgoliaMock;

    use super::*;
    use httpmock::Method::POST;
    use serde_json::json;

    #[test]
    fn batch_request_item_sets_update_action() {
        let test_json = json!({"objectID": "1", "name": "test-record", "prop": "test-prop"});
        let test_record: RecordWithObjectID = serde_json::from_value(test_json).unwrap();
        let req = BatchRequestItem::from(test_record);
        assert_eq!(req.action, BatchAction::UpdateObject);
        assert_eq!(req.body.get("objectID").unwrap(), "1");
        assert_eq!(req.body.get("name").unwrap(), "test-record");
    }

    #[test]
    fn record_without_object_id_fails_to_parse() {
        let test_json = json!({"name": "test-record", "prop": "test-prop"});
        let test_record = serde_json::from_value::<RecordWithObjectID>(test_json);
        assert!(test_record.is_err());
    }

    #[tokio::test]
    async fn upsert_records_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/batch")
                .json_body(json!({"requests": [{"action": "updateObject", "body": {"objectID": "1", "name": "foo"}}]}));
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"objectIDs": ["1"], "taskID": 123_456}));
        });

        let test_records = vec![json!({"objectID": "1", "name": "foo"})]
            .into_iter()
            .map(|v| serde_json::from_value::<RecordWithObjectID>(v).map_err(Into::into));

        let response = api
            .client
            .upsert_records("test-index", test_records)
            .await
            .unwrap();

        assert_eq!(response, vec![123_456]);
        mock.assert();
    }

    #[tokio::test]
    async fn upsert_records_with_no_records_makes_no_request() {
        let api = AlgoliaMock::new();
        let mock = api.server.mock(|when, then| {
            when.method(POST).path("/1/indexes/test-index/batch");
            then.status(200).json_body(json!({"taskID": 1}));
        });

        let response = api
            .client
            .upsert_records("test-index", std::iter::empty())
            .await
            .unwrap();

        assert_eq!(response, Vec::<u64>::new());
        mock.assert_calls(0);
    }

    #[tokio::test]
    async fn upsert_records_propagates_parse_error() {
        let api = AlgoliaMock::new();
        let mock = api.server.mock(|when, then| {
            when.method(POST).path("/1/indexes/test-index/batch");
            then.status(200).json_body(json!({"taskID": 1}));
        });

        let records = vec![Err(anyhow::anyhow!("bad record"))].into_iter();

        let error = api
            .client
            .upsert_records("test-index", records)
            .await
            .unwrap_err();

        assert!(error.to_string().contains("bad record"));
        mock.assert_calls(0);
    }

    #[tokio::test]
    async fn upsert_records_batches_at_1000_records() {
        let api = AlgoliaMock::new();

        let first_batch = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/batch")
                .is_true(|req| {
                    let body: serde_json::Value = serde_json::from_slice(req.body_ref()).unwrap();
                    body["requests"].as_array().unwrap().len() == 1000
                        && body["requests"][0]["action"] == "updateObject"
                });

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"objectIDs": [], "taskID": 1}));
        });

        let second_batch = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/batch")
                .is_true(|req| {
                    let body: serde_json::Value = serde_json::from_slice(req.body_ref()).unwrap();
                    body["requests"].as_array().unwrap().len() == 1
                });

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"objectIDs": [], "taskID": 2}));
        });

        let records = (0..1001).map(|i| {
            let record = serde_json::from_value::<RecordWithObjectID>(json!({
                "objectID": i.to_string(),
                "name": format!("record-{i}")
            }))
            .unwrap();

            Ok(record)
        });

        let task_ids = api
            .client
            .upsert_records("test-index", records)
            .await
            .unwrap();

        assert_eq!(task_ids, vec![1, 2]);
        first_batch.assert();
        second_batch.assert();
    }
}

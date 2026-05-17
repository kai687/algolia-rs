use super::{Record, RecordWithObjectID, SearchClient};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Convert each record into an appropriate batch request
pub trait IntoBatchRequestItem {
    fn into_batch_request_item(self) -> BatchRequestItem;
}

impl IntoBatchRequestItem for Record {
    fn into_batch_request_item(self) -> BatchRequestItem {
        BatchRequestItem {
            action: BatchAction::AddObject,
            body: Value::Object(self.0),
        }
    }
}

impl IntoBatchRequestItem for RecordWithObjectID {
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
    fn new<T: IntoBatchRequestItem>(records: Vec<T>) -> Self {
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
#[derive(Deserialize)]
pub struct BatchResponse {
    /// List of object IDs affected by this batch request
    #[serde(rename = "objectIDs")]
    object_ids: Vec<String>,
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
    async fn batch<R: IntoBatchRequestItem>(
        &self,
        index_name: &str,
        records: Vec<R>,
    ) -> Result<BatchResponse> {
        let request_url = format!("/indexes/{index_name}/batch");
        let batch_request = BatchRequest::new(records);
        self.post(&request_url, Some(&batch_request)).await
    }

    /// Push new records to an Algolia index
    /// Records will be added with an auto-generated `objectID`
    pub async fn push_records(
        &self,
        index_name: &str,
        records: Vec<Record>,
    ) -> Result<BatchResponse> {
        self.batch(index_name, records).await
    }

    /// Upsert new records to an Algolia index
    /// Records with matching `objectID` will be replaced.
    pub async fn upsert_records(
        &self,
        index_name: &str,
        records: Vec<RecordWithObjectID>,
    ) -> Result<BatchResponse> {
        self.batch(index_name, records).await
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

        let test_json = json!([{
            "name": "foo",
            "prop": "foo-prop",
        }, {
            "name": "bar",
            "prop": "bar-prop",
        }]);
        let test_records: Vec<Record> = serde_json::from_value(test_json).unwrap();

        client
            .push_records("test-index", test_records)
            .await
            .unwrap();
        mock.assert();
    }
}

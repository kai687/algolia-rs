use super::{SearchClient, UpdatedAtResponse};
use anyhow::Result;
use serde::Serialize;

/// Allowed scopes for the `copy` operation
#[derive(Clone, Debug, Serialize, clap::ValueEnum)]
#[serde(rename_all = "camelCase")]
pub enum OperationScope {
    Settings,
    Synonyms,
    Rules,
}

/// `IndexOperation` body for copying indices
#[derive(Serialize)]
pub struct CopyOperation {
    /// Destination index
    pub destination: String,
    /// What to copy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Vec<OperationScope>>,
}

/// `IndexOperation` body for moving indices
#[derive(Serialize)]
pub struct MoveOperation {
    /// Destination index
    pub destination: String,
}

/// The request body for copying or moving an index
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum IndexOperation {
    Copy(CopyOperation),
    Move(MoveOperation),
}

impl SearchClient {
    /// Copy an index
    pub async fn copy_index(
        &self,
        index_name: &str,
        data: &CopyOperation,
    ) -> Result<UpdatedAtResponse> {
        let request_url = format!("/indexes/{index_name}/operation");

        self.post(&request_url, Some(&data)).await
    }

    /// Move (rename) an index
    pub async fn move_index(
        &self,
        index_name: &str,
        data: &MoveOperation,
    ) -> Result<UpdatedAtResponse> {
        let request_url = format!("/indexes/{index_name}/operation");

        self.post(&request_url, Some(&data)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use serde_json::json;

    #[test]
    fn copy_operation_body_serializes_correctly() {
        let op = IndexOperation::Copy(CopyOperation {
            destination: "new_index".to_string(),
            scope: None,
        });

        let op_json = serde_json::to_value(&op).unwrap();
        assert_eq!(
            op_json,
            json!({
                "type": "copy",
                "destination": "new_index"
            })
        );
    }

    #[test]
    fn copy_operation_body_with_scopes_serializes_correctly() {
        let op = IndexOperation::Copy(CopyOperation {
            destination: "new_index".to_string(),
            scope: Some(vec![OperationScope::Settings, OperationScope::Synonyms]),
        });

        let op_json = serde_json::to_value(&op).unwrap();
        assert_eq!(
            op_json,
            json!({
                "type": "copy",
                "destination": "new_index",
                "scope": ["settings", "synonyms"]
            })
        );
    }

    #[test]
    fn move_index_body_serializes_correctly() {
        let op = IndexOperation::Move(MoveOperation {
            destination: "new_index".to_string(),
        });

        let op_json = serde_json::to_value(&op).unwrap();
        assert_eq!(
            op_json,
            json!({
                "type": "move",
                "destination": "new_index"
            })
        );
    }

    #[tokio::test]
    async fn copy_indices_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/1/indexes/test-index/operation");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "taskID": 123_456,
                    "updatedAt": "2026-04-29T08:50:12.848Z",
                }));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let data = CopyOperation {
            destination: "test-destination".to_string(),
            scope: None,
        };

        let response = client.copy_index("test-index", &data).await.unwrap();
        mock.assert();
        assert_eq!(response.task_id, 123_456);
        assert!(!response.updated_at.is_empty());
    }

    #[tokio::test]
    async fn move_index_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST).path("/1/indexes/test-index/operation");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "taskID": 123_456,
                    "updatedAt": "2026-04-29T08:50:12.848Z",
                }));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let data = MoveOperation {
            destination: "test-destination".to_string(),
        };

        let response = client.move_index("test-index", &data).await.unwrap();
        mock.assert();
        assert_eq!(response.task_id, 123_456);
        assert!(!response.updated_at.is_empty());
    }
}

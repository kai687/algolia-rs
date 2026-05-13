use super::SearchClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// The response for the 'list indices' endpoint
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListIndicesResponse {
    pub items: Vec<ListIndexItem>,
    pub nb_pages: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListIndexItem {
    /// Index name
    pub name: String,
    /// Date and time when the index was created
    pub created_at: String,
    /// Date and time when the index was last updated
    pub updated_at: String,
    /// Number of records in this index
    pub entries: u32,
    /// Size of the minified index in bytes
    pub data_size: u64,
    /// Size of the index file on disk in bytes
    pub file_size: u64,
    /// How long it took to build the index in seconds
    pub last_build_time_s: u32,
    /// Number of pending tasks for this index
    pub number_of_pending_tasks: u32,
    /// (Replica indices only) Name of the primary index
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    /// (Primary indices with replicas only) Replica index names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<Vec<String>>,
    /// Whether this index has pending tasks
    pub pending_task: bool,
    /// Virtual replica indices only (virtual is a reserved keyword)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#virtual: Option<bool>,
}

impl SearchClient {
    /// List all indices
    pub async fn list_indices(&self) -> Result<ListIndicesResponse> {
        self.get("/indexes").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    #[tokio::test]
    async fn list_indices_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/1/indexes");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    serde_json::json!({
                        "items": [
                            {
                                "name": "Test",
                                "entries": 0,
                                "createdAt": "2026-04-29T08:50:12.848Z",
                                "updatedAt": "2026-04-29T08:50:12.848Z",
                                "dataSize": 1,
                                "fileSize": 1,
                                "lastBuildTimeS": 1,
                                "numberOfPendingTasks": 0,
                                "pendingTask": false
                            }
                        ],
                        "nbPages": 1
                    })
                    .to_string(),
                );
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        client.list_indices().await.unwrap();

        mock.assert();
    }
}

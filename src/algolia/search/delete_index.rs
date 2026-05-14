use super::SearchClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedAtResponse {
    /// The task ID for waiting
    #[serde(rename = "taskID")]
    pub task_id: u64,
    /// The time when the deletion was requested
    pub deleted_at: String,
}

impl fmt::Display for DeletedAtResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} Task {} queued at {}",
            "🗸 ".green(),
            self.task_id.to_string().dimmed(),
            self.deleted_at.dimmed()
        )
    }
}

impl SearchClient {
    /// Delete an index
    pub async fn delete_index(&self, index_name: &str) -> Result<DeletedAtResponse> {
        let request_url = format!("/indexes/{index_name}");
        self.delete(&request_url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::DELETE;
    use httpmock::MockServer;
    use serde_json::json;

    #[tokio::test]
    async fn delete_index() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(DELETE).path("/1/indexes/test-index");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"taskID": 123456, "deletedAt": "2026-05-14T19:44"}));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let response = client.delete_index("test-index").await.unwrap();
        mock.assert();
        assert_eq!(response.task_id, 123_456);
        assert!(!response.deleted_at.is_empty());
    }
}

use std::time::Duration;

use super::SearchClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

#[derive(Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Published,
    NotPublished,
}

#[derive(Deserialize, Serialize)]
pub struct CheckTaskStatusResponse {
    /// Task's status
    pub status: TaskStatus,
}

impl SearchClient {
    /// Check for task status
    async fn check_task_status(
        &self,
        index_name: &str,
        task_id: u64,
    ) -> Result<CheckTaskStatusResponse> {
        let request_url = format!("/indexes/{index_name}/task/{task_id}");
        self.get(&request_url).await
    }

    /// Wait until an indexing task is complete
    pub async fn wait_for_task(&self, index_name: &str, task_id: u64) -> Result<()> {
        // Hard-code max retries since I can't set a default value in this function
        const MAX_RETRIES: u16 = 10;

        for retry in 0..MAX_RETRIES {
            let response = self.check_task_status(index_name, task_id).await?;

            if let TaskStatus::Published = response.status {
                return Ok(());
            }

            // Exponential backoff until 5 seconds
            let delay_ms = (retry * 200).min(5000);
            sleep(Duration::from_millis(u64::from(delay_ms))).await;
        }

        Err(anyhow::anyhow!(
            "Task {task_id} did not complete after {MAX_RETRIES} retries."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    #[tokio::test]
    async fn check_task_status_makes_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/1/indexes/test-index/task/123456");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({"status": "published"}));
        });

        let index_name = "test-index";
        let task_id: u64 = 123_456;

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let response = client.check_task_status(index_name, task_id).await.unwrap();

        mock.assert();
        assert_eq!(response.status, TaskStatus::Published);
    }
}

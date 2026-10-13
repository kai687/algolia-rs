use std::time::{Duration, Instant};

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

/// Give up waiting for a task after this long
pub const MAX_WAIT: Duration = Duration::from_mins(2);
/// Longest pause between two status checks
const MAX_DELAY: Duration = Duration::from_secs(5);

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

    /// Wait until an indexing task is complete.
    ///
    /// Polls with a backoff capped at 5 seconds and gives up after
    /// [`MAX_WAIT`].
    pub async fn wait_for_task(&self, index_name: &str, task_id: u64) -> Result<()> {
        self.wait_for_task_with_timeout(index_name, task_id, MAX_WAIT)
            .await
    }

    /// Wait until an indexing task is complete, or `timeout` elapses.
    pub async fn wait_for_task_with_timeout(
        &self,
        index_name: &str,
        task_id: u64,
        timeout: Duration,
    ) -> Result<()> {
        let start = Instant::now();
        let mut retry: u32 = 0;

        loop {
            let response = self.check_task_status(index_name, task_id).await?;

            if let TaskStatus::Published = response.status {
                return Ok(());
            }

            if start.elapsed() >= timeout {
                return Err(anyhow::anyhow!(
                    "Task {task_id} on index {index_name} did not complete within {} seconds.",
                    timeout.as_secs()
                ));
            }

            // Linear backoff until 5 seconds
            let delay = Duration::from_millis(u64::from(retry) * 200).min(MAX_DELAY);
            sleep(delay).await;
            retry += 1;
        }
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

    #[tokio::test]
    async fn wait_for_task_returns_when_published() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/1/indexes/test-index/task/1");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({"status": "published"}));
        });
        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        client.wait_for_task("test-index", 1).await.unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn wait_for_task_fails_after_timeout() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/1/indexes/test-index/task/1");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(serde_json::json!({"status": "notPublished"}));
        });
        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let err = client
            .wait_for_task_with_timeout("test-index", 1, Duration::from_millis(300))
            .await
            .unwrap_err();

        assert!(err.to_string().contains("did not complete"));
        assert!(mock.calls() >= 2);
    }
}

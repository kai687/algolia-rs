use super::SearchClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedAtResponse {
    /// The task ID for waiting
    #[serde(rename = "taskID")]
    pub task_id: u64,
    /// The time when the deletion was requested
    pub deleted_at: String,
}

impl SearchClient {
    /// Delete an index
    pub async fn delete_index(&self, index_name: &str) -> Result<DeletedAtResponse> {
        let request_url = format!("/indexes/{index_name}");
        self.delete(&request_url).await
    }
} 

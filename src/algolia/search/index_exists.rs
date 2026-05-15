use super::SearchClient;
use anyhow::Result;

impl SearchClient {
    /// Check if an index exists (check if the Get settings op succeeds)
    pub async fn index_exists(&self, index_name: &str) -> Result<bool> {
        let request_url = format!("/indexes/{index_name}/settings");
        self.exists(&request_url).await
    }
}

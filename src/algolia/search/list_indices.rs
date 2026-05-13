use anyhow::Result;
use super::SearchClient;

impl SearchClient {
    /// List all indices
    pub async fn list_indices(&self) -> Result<()> {
        self.get("/indexes").await
    }
}

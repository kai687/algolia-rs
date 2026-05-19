use super::SearchClient;
use anyhow::Result;

/// Deal with index settings as raw JSON
pub type IndexSettings = serde_json::Value;

impl SearchClient {
    pub async fn get_settings(&self, index_name: &str) -> Result<IndexSettings> {
        let request_url = format!("/indexes/{index_name}/settings");

        self.get(&request_url).await
    }
}

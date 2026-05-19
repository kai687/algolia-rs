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

#[cfg(test)]
mod tests {
    use crate::algolia::search::test_support::AlgoliaMock;
    use httpmock::Method::GET;

    #[tokio::test]
    async fn get_settings_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(
            GET,
            "/1/indexes/test-index/settings",
            200,
            serde_json::json!({"distinct": true}),
        );

        api.client.get_settings("test-index").await.unwrap();

        mock.assert();
    }
}

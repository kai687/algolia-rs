use super::{SearchClient, UpdatedAtResponse};
use anyhow::Result;

/// Index settings as raw JSON. Keys are kept sorted by `serde_json`.
pub type IndexSettings = serde_json::Value;

impl SearchClient {
    /// Fetch the settings of an index
    pub async fn get_settings(&self, index_name: &str) -> Result<IndexSettings> {
        self.get(&format!("/indexes/{index_name}/settings")).await
    }

    /// Replace the settings of an index
    pub async fn set_settings(
        &self,
        index_name: &str,
        settings: &IndexSettings,
    ) -> Result<UpdatedAtResponse> {
        self.put(&format!("/indexes/{index_name}/settings"), settings)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::api::test_support::AlgoliaMock;
    use httpmock::Method::{GET, PUT};
    use serde_json::json;

    #[tokio::test]
    async fn get_settings_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(
            GET,
            "/1/indexes/test-index/settings",
            200,
            json!({"distinct": true}),
        );

        let settings = api.client.get_settings("test-index").await.unwrap();

        mock.assert();
        assert_eq!(settings, json!({"distinct": true}));
    }

    #[tokio::test]
    async fn set_settings_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.server.mock(|when, then| {
            when.method(PUT)
                .path("/1/indexes/test-index/settings")
                .json_body(json!({"distinct": true}));

            then.status(200)
                .json_body(json!({"taskID": 123_456, "updatedAt": "2026-05-20T19:35"}));
        });

        let settings = json!({"distinct": true});
        let response = api
            .client
            .set_settings("test-index", &settings)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(response.task_id, 123_456);
        assert_eq!(response.updated_at, "2026-05-20T19:35");
    }
}

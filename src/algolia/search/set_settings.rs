use super::{SearchClient, UpdatedAtResponse};
use anyhow::Result;

/// Deal with index settings as raw JSON
pub type IndexSettings = serde_json::Value;

impl SearchClient {
    pub async fn set_settings(
        &self,
        index_name: &str,
        settings: &IndexSettings,
    ) -> Result<UpdatedAtResponse> {
        let request_url = format!("/indexes/{index_name}/settings");

        self.put(&request_url, &settings).await
    }
}

#[cfg(test)]
mod tests {
    use crate::algolia::search::test_support::AlgoliaMock;
    use httpmock::Method::PUT;
    use serde_json::json;

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

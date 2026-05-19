use crate::algolia::search::AlgoliaApiError;

use super::SearchClient;
use anyhow::Result;

impl SearchClient {
    /// Check if an index exists (check if the Get settings op succeeds)
    pub async fn index_exists(&self, index_name: &str) -> Result<bool> {
        // self.exists(&request_url).await
        match self.get_settings(index_name).await {
            Ok(_) => Ok(true),
            Err(err) => {
                if let Some(api_err) = err.downcast_ref::<AlgoliaApiError>()
                    && api_err.status == 404
                {
                    return Ok(false);
                }
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::algolia::search::test_support::AlgoliaMock;
    use httpmock::Method::GET;

    #[tokio::test]
    async fn index_exists_returns_true() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(
            GET,
            "/1/indexes/test-index/settings",
            200,
            serde_json::json!({"foo": "bar"}),
        );

        let response = api.client.index_exists("test-index").await.unwrap();

        assert!(response);
        mock.assert();
    }

    #[tokio::test]
    async fn index_exists_returns_false() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(
            GET,
            "/1/indexes/test-index/settings",
            404,
            serde_json::json!({"status": 404, "message": "Index not found"}),
        );

        let response = api.client.index_exists("test-index").await.unwrap();
        assert!(!response);

        mock.assert();
    }

    #[tokio::test]
    async fn index_exists_returns_error() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(
            GET,
            "/1/indexes/test-index/settings",
            503,
            serde_json::json!({"message": "Bad request", "status": 400}),
        );

        let result = api.client.index_exists("test-index").await;
        assert!(result.is_err());

        mock.assert();
    }
}

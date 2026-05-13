use anyhow::{Context, Result};
use reqwest::{Client, header};
use serde::de::DeserializeOwned;

pub mod list_indices;

/// Client for requests to the Algolia Search API
pub struct SearchClient {
    /// The HTTP client
    http: Client,
    /// The base URL for making requests
    base_url: String,
}

impl SearchClient {
    /// Create a new Search API client
    pub fn new(app_id: &str, api_key: &str) -> Result<Self> {
        let base_url = format!("https://{}-dsn.algolia.net/1", &app_id);
        Self::new_with_base_url(app_id, api_key, base_url)
    }
    /// Constructor for easier testing
    pub fn new_with_base_url(app_id: &str, api_key: &str, base_url: String) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "x-algolia-application-id",
            header::HeaderValue::from_str(&app_id)?,
        );
        headers.insert(
            "x-algolia-api-key",
            header::HeaderValue::from_str(&api_key)?,
        );

        let http = Client::builder()
            .default_headers(headers)
            .build()
            .context("Can't create search client")?;

        Ok(Self { http, base_url })
    }

    /// Wrapper for GET requests (prints JSON response)
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);

        let response = self
            .http
            .get(request_url)
            .send()
            .await
            .context("Can't make HTTP GET request")?;

        Ok(response
            .json::<T>()
            .await
            .context("Can't deserialize response.")?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    #[tokio::test]
    async fn get_sends_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/1/testing");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"items": []}"#);
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        client.get::<serde_json::Value>("/testing").await.unwrap();

        mock.assert();
    }
}

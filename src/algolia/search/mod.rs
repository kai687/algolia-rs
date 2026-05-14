use std::fmt;

use anyhow::{Context, Result};
use chrono::DateTime;
use colored::Colorize;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub mod delete_index;
pub mod list_indices;
pub mod operation_index;
pub mod wait_for_indexing_task;

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
            header::HeaderValue::from_str(app_id)?,
        );
        headers.insert("x-algolia-api-key", header::HeaderValue::from_str(api_key)?);

        let http = Client::builder()
            .default_headers(headers)
            .build()
            .context("Can't create search client")?;

        Ok(Self { http, base_url })
    }

    /// Wrapper for generic request sends and response serialization
    async fn send<T: DeserializeOwned>(&self, request: reqwest::RequestBuilder) -> Result<T> {
        let response = request.send().await.context("Can't make HTTP request")?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {status}: {body}");
        }

        response
            .json::<T>()
            .await
            .context("Can't deserialize response")
    }

    /// Wrapper for GET requests (returns JSON response)
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);
        self.send(self.http.get(request_url)).await
    }

    /// Wrapper for POST requests (Returns JSON response)
    pub async fn post<T: DeserializeOwned, D: Serialize>(
        &self,
        path: &str,
        data: Option<&D>,
    ) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);
        self.send(self.http.post(request_url).json(&data)).await
    }

    /// Wrapper for DELETE requests (Returns JSON response)
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);
        self.send(self.http.delete(request_url)).await
    }
}

/// Shared response with Task ID
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedAtResponse {
    #[serde(rename = "taskID")]
    pub task_id: u64,
    pub updated_at: String,
}

impl fmt::Display for UpdatedAtResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dt = DateTime::parse_from_rfc3339(&self.updated_at).map_err(|_| fmt::Error)?;
        write!(
            f,
            "{} Task {} queued at {}",
            "⧖".green(),
            self.task_id.to_string().dimmed(),
            dt.format("%Y-%m-%d %H:%M:%S").to_string().dimmed()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::{DELETE, GET, POST};
    use httpmock::MockServer;
    use serde_json::json;

    #[tokio::test]
    async fn get_sends_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/1/testing")
                .header("x-algolia-application-id", "test-app")
                .header("x-algolia-api-key", "test-key");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"items": []}));
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

    #[tokio::test]
    async fn get_fails_with_good_error() {
        let server = MockServer::start();

        server.mock(|when, then| {
            when.method(GET).path("/1/testing");
            then.status(404)
                .header("content-type", "application/json")
                .json_body(json!({"message": "Index not found"}));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let result = client.get::<serde_json::Value>("/testing").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Index not found"));
    }

    #[derive(Serialize)]
    struct TestData {
        foo: String,
        bar: String,
    }

    #[tokio::test]
    async fn post_sends_request_with_data() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/1/testing")
                .header("x-algolia-application-id", "test-app")
                .header("x-algolia-api-key", "test-key");
            then.status(201)
                .header("content-type", "application/json")
                .json_body(json!({"message": "created"}));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let data = TestData {
            foo: "foo".to_string(),
            bar: "bar".to_string(),
        };

        client
            .post::<serde_json::Value, TestData>("/testing", Some(&data))
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn post_sends_request_without_data() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/1/testing")
                .header("x-algolia-application-id", "test-app")
                .header("x-algolia-api-key", "test-key");
            then.status(201)
                .header("content-type", "application/json")
                .json_body(json!({"message": "created"}));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        client
            .post::<serde_json::Value, serde_json::Value>("/testing", None)
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn delete_sends_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(DELETE)
                .path("/1/testing")
                .header("x-algolia-application-id", "test-app")
                .header("x-algolia-api-key", "test-key");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"status": "deleted"}));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        client
            .delete::<serde_json::Value>("/testing")
            .await
            .unwrap();

        mock.assert();
    }
}

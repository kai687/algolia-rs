use std::fmt;

use anyhow::{Context, Result};
use chrono::DateTime;
use colored::Colorize;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

pub mod delete_index;
// pub mod index_exists;
pub mod list_indices;
pub mod list_records;
pub mod operation_index;
pub mod push_records;
pub mod wait_for_indexing_task;

/// Represent an API error
#[derive(Debug, Deserialize, thiserror::Error)]
pub struct AlgoliaApiError {
    pub message: String,
    pub status: u16,
}

impl fmt::Display for AlgoliaApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "API error {}: {}", self.status, self.message)
    }
}

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

        if status.is_success() {
            return response
                .json::<T>()
                .await
                .context("Can't deserialize response");
        }

        let body = response.text().await.unwrap_or_default();

        match serde_json::from_str::<AlgoliaApiError>(&body) {
            Ok(err) => Err(AlgoliaApiError {
                status: status.as_u16(),
                message: err.message,
            }
            .into()),
            Err(_) => Err(anyhow::anyhow!("HTTP {status}: {body}")),
        }
    }

    /// Wrapper for GET requests (returns JSON response)
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);
        self.send(self.http.get(request_url)).await
    }

    /// Wrapper for POST requests (Returns JSON response)
    async fn post<T: DeserializeOwned, D: Serialize>(
        &self,
        path: &str,
        data: Option<&D>,
    ) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);
        self.send(self.http.post(request_url).json(&data)).await
    }

    /// Wrapper for DELETE requests (Returns JSON response)
    async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);
        self.send(self.http.delete(request_url)).await
    }
}

/// Algolia record (without object ID)
#[derive(Debug, Serialize, Deserialize)]
pub struct Record(Map<String, Value>);

/// Algolia record with object ID
#[derive(Debug, Serialize, Deserialize)]
pub struct RecordWithObjectID {
    /// A unique identifier for the record
    #[serde(rename = "objectID")]
    pub object_id: String,

    /// Record attributes
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

/// Try to 'upcast' a simple record to one with `objectID`
impl TryFrom<Record> for RecordWithObjectID {
    type Error = anyhow::Error;

    fn try_from(record: Record) -> Result<Self> {
        serde_json::from_value(Value::Object(record.0)).map_err(Into::into)
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
    use test_support::AlgoliaMock;

    use super::*;
    use httpmock::Method::{DELETE, GET, POST};
    use serde_json::json;

    #[tokio::test]
    async fn get_request_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.mock_response(GET, "/1/testing", 200, json!({}));

        api.client
            .get::<serde_json::Value>("/testing")
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn get_request_fails_with_algolia_api_error() {
        let api = AlgoliaMock::new();
        let mock = api.mock_response(
            GET,
            "/1/testing",
            404,
            json!({"message": "Index not found", "status": 404}),
        );

        let error = api
            .client
            .get::<serde_json::Value>("/testing")
            .await
            .unwrap_err();

        let api_err = error
            .downcast_ref::<AlgoliaApiError>()
            .expect("expected AlgoliaApiError");

        assert!(matches!(api_err, AlgoliaApiError { status: 404, .. }));
        mock.assert();
    }

    #[tokio::test]
    async fn get_request_fails_with_different_error() {
        let api = AlgoliaMock::new();
        let mock = api.mock_response(
            GET,
            "/1/testing",
            503,
            json!({ "text": "Unstructured error" }),
        );

        let error = api
            .client
            .get::<serde_json::Value>("/testing")
            .await
            .unwrap_err();
        let api_err = error.downcast_ref::<AlgoliaApiError>();

        assert!(api_err.is_none());
        mock.assert();
    }

    #[derive(Serialize)]
    struct TestData {
        foo: String,
        bar: String,
    }

    #[tokio::test]
    async fn post_sends_request_with_data() {
        let api = AlgoliaMock::new();
        let mock = api.mock_response(POST, "/1/testing", 201, json!({}));
        let data = TestData {
            foo: "foo".to_string(),
            bar: "bar".to_string(),
        };

        api.client
            .post::<serde_json::Value, TestData>("/testing", Some(&data))
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn post_sends_request_without_data() {
        let api = AlgoliaMock::new();
        let mock = api.mock_response(POST, "/1/testing", 201, json!({}));

        api.client
            .post::<serde_json::Value, serde_json::Value>("/testing", None)
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn delete_request_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.mock_response(DELETE, "/1/testing", 200, json!({}));

        api.client
            .delete::<serde_json::Value>("/testing")
            .await
            .unwrap();
        mock.assert();
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::SearchClient;
    use httpmock::{Method, Mock, MockServer};

    pub(crate) struct AlgoliaMock {
        pub server: MockServer,
        pub client: SearchClient,
    }

    impl AlgoliaMock {
        pub(crate) fn new() -> Self {
            let server = MockServer::start();
            let client = SearchClient::new_with_base_url(
                "test-app",
                "test-key",
                format!("{}/1", server.base_url()),
            )
            .unwrap();

            Self { server, client }
        }

        pub(crate) fn mock_response(
            &self,
            method: Method,
            path: &str,
            status: u16,
            body: serde_json::Value,
        ) -> Mock<'_> {
            self.server.mock(|when, then| {
                when.method(method)
                    .path(path)
                    .header("x-algolia-application-id", "test-app")
                    .header("x-algolia-api-key", "test-key");

                then.status(status)
                    .header("content-type", "application/json")
                    .json_body(body);
            })
        }
    }
}

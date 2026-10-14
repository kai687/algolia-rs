//! Thin client for the Algolia Search REST API.
//!
//! This layer only speaks HTTP and JSON. It returns typed responses and
//! never formats output. Presentation lives in [`crate::cmd`].

use std::fmt;

use anyhow::{Context, Result};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub mod indices;
pub mod records;
pub mod settings;
pub mod tasks;

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

/// Response of write operations that return a task ID and a timestamp
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedAtResponse {
    #[serde(rename = "taskID")]
    pub task_id: u64,
    pub updated_at: String,
}

/// Client for requests to the Algolia Search API
pub struct SearchClient {
    /// The HTTP client
    http: Client,
    /// The base URL for making requests
    base_url: String,
}

/// Base URL of the primary host.
///
/// The `-dsn` host is meant for reads and can reject writes with
/// "This host is read-only" (HTTP 400), so every request uses the primary host.
fn default_base_url(app_id: &str) -> String {
    format!("https://{app_id}.algolia.net/1")
}

impl SearchClient {
    /// Create a new Search API client
    pub fn new(app_id: &str, api_key: &str) -> Result<Self> {
        Self::new_with_base_url(app_id, api_key, default_base_url(app_id))
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

        // Try to deserialize 'known' errors from the API
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

    /// Wrapper for POST requests with a request body (Returns JSON response)
    async fn post<T: DeserializeOwned, D: Serialize>(&self, path: &str, data: &D) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);
        let request = self.http.post(request_url).json(data);

        self.send(request).await
    }

    /// Wrapper for PUT requests (require request body) (Returns JSON response)
    async fn put<T: DeserializeOwned, D: Serialize>(&self, path: &str, data: &D) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);
        let request = self.http.put(request_url).json(data);

        self.send(request).await
    }

    /// Wrapper for DELETE requests (Returns JSON response)
    async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let request_url = format!("{}{}", self.base_url, path);
        self.send(self.http.delete(request_url)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::{DELETE, GET, POST, PUT};
    use serde_json::json;
    use test_support::AlgoliaMock;

    #[tokio::test]
    async fn get_request_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(GET, "/1/testing", 200, json!({}));

        api.client
            .get::<serde_json::Value>("/testing")
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn get_request_fails_with_algolia_api_error() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(
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
        let mock = api.json_response(
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

    #[tokio::test]
    async fn error_response_includes_status_and_body() {
        let api = AlgoliaMock::new();
        let mock = api.text_response(GET, "/1/testing", 503, "Service unavailable");

        let error = api
            .client
            .get::<serde_json::Value>("/testing")
            .await
            .unwrap_err();

        let error_message = error.to_string();
        assert!(error_message.contains("HTTP 503"));
        assert!(error_message.contains("Service unavailable"));

        mock.assert();
    }

    #[tokio::test]
    async fn get_request_fails_for_non_json_response() {
        let api = AlgoliaMock::new();
        let mock = api.text_response(GET, "/1/testing", 200, "not json");

        let error = api
            .client
            .get::<serde_json::Value>("/testing")
            .await
            .unwrap_err();

        assert!(error.to_string().contains("Can't deserialize response"));
        mock.assert();
    }

    #[derive(Serialize)]
    struct TestData {
        foo: String,
        bar: String,
    }

    #[tokio::test]
    async fn post_request_with_data() {
        let api = AlgoliaMock::new();

        let mock = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/testing")
                .json_body(json!({"foo": "foo", "bar": "bar"}));
            then.status(201).json_body(json!({}));
        });

        let data = TestData {
            foo: "foo".to_string(),
            bar: "bar".to_string(),
        };

        api.client
            .post::<serde_json::Value, TestData>("/testing", &data)
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn put_request_succeeds() {
        let api = AlgoliaMock::new();

        let mock = api.server.mock(|when, then| {
            when.method(PUT)
                .path("/1/testing")
                .json_body(json!({"foo": "foo", "bar": "bar"}));
            then.status(200).json_body(json!({}));
        });

        let data = json!({"foo": "foo", "bar": "bar"});

        api.client
            .put::<serde_json::Value, serde_json::Value>("/testing", &data)
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn delete_request_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(DELETE, "/1/testing", 200, json!({}));

        api.client
            .delete::<serde_json::Value>("/testing")
            .await
            .unwrap();

        mock.assert();
    }

    #[test]
    fn default_base_url_uses_primary_host() {
        assert_eq!(default_base_url("APPID"), "https://APPID.algolia.net/1");
    }

    #[test]
    fn new_with_base_url_fails_for_invalid_application_id() {
        let result = SearchClient::new_with_base_url(
            "bad\nvalue",
            "test-key",
            "http://localhost".to_string(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn new_with_base_url_fails_for_invalid_api_key() {
        let result = SearchClient::new_with_base_url(
            "test-app",
            "bad\nvalue",
            "http://localhost".to_string(),
        );

        assert!(result.is_err());
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

        pub(crate) fn json_response(
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

        pub(crate) fn text_response(
            &self,
            method: Method,
            path: &str,
            status: u16,
            body: &str,
        ) -> Mock<'_> {
            self.server.mock(|when, then| {
                when.method(method)
                    .path(path)
                    .header("x-algolia-application-id", "test-app")
                    .header("x-algolia-api-key", "test-key");

                then.status(status)
                    .header("content-type", "application/json")
                    .body(body);
            })
        }
    }
}

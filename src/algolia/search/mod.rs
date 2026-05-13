use anyhow::{Context, Result};
use reqwest::{Client, header};

pub mod list_indices;

pub struct SearchClient {
    http: Client,
    base_url: String,
}

impl SearchClient {
    /// Create a new Search API client
    pub fn new(app_id: &str, api_key: &str) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "x-algolia-application-id",
            header::HeaderValue::from_str(&app_id)?,
        );
        headers.insert(
            "x-algolia-api-key",
            header::HeaderValue::from_str(&api_key)?,
        );

        let base_url = format!("https://{}-dsn.algolia.net/1", &app_id);

        let http = Client::builder().default_headers(headers).build().with_context(|| "Can't create search client")?;

        Ok(Self { http, base_url })
    }

    /// Wrapper for GET requests (prints JSON response)
    async fn get(&self, path: &str) -> Result<()> {
        let request_url = format!("{}{}", self.base_url, path);

        let response = self.http.get(request_url).send().await.with_context(|| "Can't make HTTP GET request")?;
        let body: serde_json::Value = response.json().await.with_context(|| "Can't get the response as JSON")?;

        println!("{}", serde_json::to_string_pretty(&body).with_context(|| "Can't pretty print JSON")?);

        Ok(())
    }
}

use anyhow::{Context, Result};

/// Helper function for Algolia credentials
#[derive(Debug)]
pub struct Credentials {
    /// The Algolia application ID
    pub app_id: String,
    /// The Algolia API key for authenticating requests
    pub api_key: String,
}

impl Credentials {
    /// Create a new Credentials instance
    fn new(app_id: Option<String>, api_key: Option<String>) -> Result<Self> {
        let app_id = app_id.context("Missing credential: ALGOLIA_APPLICATION_ID")?;
        let api_key = api_key.context("Missing credential: ALGOLIA_API_KEY")?;

        Ok(Self { app_id, api_key })
    }

    /// Create a new Credentials instance from environment variables
    pub fn from_env() -> Result<Self> {
        Self::new(
            std::env::var("ALGOLIA_APPLICATION_ID").ok(),
            std::env::var("ALGOLIA_API_KEY").ok(),
        )
    }
}

#[test]
fn new_credentials() {
    let credentials =
        Credentials::new(Some("test-app".to_string()), Some("test-key".to_string())).unwrap();
    assert_eq!(credentials.app_id, "test-app");
    assert_eq!(credentials.api_key, "test-key");
}

#[test]
fn new_errors_without_application_id() {
    let err = Credentials::new(None, Some("api_key".to_string())).unwrap_err();
    assert!(err.to_string().contains("Missing credential"));
    assert!(err.to_string().contains("ALGOLIA_APPLICATION_ID"));
}

#[test]
fn new_errors_without_api_key() {
    let err = Credentials::new(Some("application_id".to_string()), None).unwrap_err();
    assert!(err.to_string().contains("Missing credential"));
    assert!(err.to_string().contains("ALGOLIA_API_KEY"));
}

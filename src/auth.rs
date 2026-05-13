use anyhow::{Context, Result};

/// Helper function for Algolia credentials
pub struct Credentials {
    /// The Algolia application ID
    pub app_id: String,
    /// The Algolia API key for authenticating requests
    pub api_key: String,
}

impl Credentials {
    pub fn from_env() -> Result<Self> {

        let app_id = std::env::var("ALGOLIA_APPLICATION_ID")
            .with_context(|| "Missing environment variable: `ALGOLIA_APPLICATION_ID`")?;
        let api_key = std::env::var("ALGOLIA_API_KEY")
            .with_context(|| "Missing environment variable: `ALGOLIA_API_KEY`")?;

        Ok(Self { app_id, api_key })
    }
}

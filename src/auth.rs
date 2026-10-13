use anyhow::{Result, bail};

/// Algolia credentials
#[derive(Debug)]
pub struct Credentials {
    /// The Algolia application ID
    pub app_id: String,
    /// The Algolia API key for authenticating requests
    pub api_key: String,
}

pub const APP_ID: &str = "ALGOLIA_APPLICATION_ID";
pub const API_KEY: &str = "ALGOLIA_API_KEY";

impl Credentials {
    /// Load credentials from the environment.
    ///
    /// This CLI targets CI, so environment variables are the only source.
    pub fn load() -> Result<Self> {
        Self::from_env(std::env::var(APP_ID).ok(), std::env::var(API_KEY).ok())
    }

    fn from_env(app_id: Option<String>, api_key: Option<String>) -> Result<Self> {
        let app_id = app_id.filter(|value| !value.trim().is_empty());
        let api_key = api_key.filter(|value| !value.trim().is_empty());

        match (app_id, api_key) {
            (Some(app_id), Some(api_key)) => Ok(Self { app_id, api_key }),
            (None, Some(_)) => bail!("Missing environment variable {APP_ID}"),
            (Some(_), None) => bail!("Missing environment variable {API_KEY}"),
            (None, None) => bail!("Missing environment variables {APP_ID} and {API_KEY}"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[allow(clippy::unnecessary_wraps)]
    fn s(value: &str) -> Option<String> {
        Some(value.to_string())
    }

    #[test]
    fn loads_when_both_set() {
        let creds = Credentials::from_env(s("test-app"), s("test-key")).unwrap();

        assert_eq!(creds.app_id, "test-app");
        assert_eq!(creds.api_key, "test-key");
    }

    #[test]
    fn fails_when_app_id_missing() {
        let err = Credentials::from_env(None, s("test-key")).unwrap_err();
        assert!(err.to_string().contains(APP_ID));
        assert!(!err.to_string().contains(API_KEY));
    }

    #[test]
    fn fails_when_api_key_missing() {
        let err = Credentials::from_env(s("test-app"), None).unwrap_err();
        assert!(err.to_string().contains(API_KEY));
        assert!(!err.to_string().contains(APP_ID));
    }

    #[test]
    fn fails_when_both_missing() {
        let err = Credentials::from_env(None, None).unwrap_err();
        assert!(err.to_string().contains(APP_ID));
        assert!(err.to_string().contains(API_KEY));
    }

    #[test]
    fn treats_empty_as_missing() {
        let err = Credentials::from_env(s(""), s("  ")).unwrap_err();
        assert!(err.to_string().contains(APP_ID));
        assert!(err.to_string().contains(API_KEY));
    }
}

use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

/// Algolia credentials
#[derive(Debug)]
pub struct Credentials {
    /// The Algolia application ID
    pub app_id: String,
    /// The Algolia API key for authenticating requests
    pub api_key: String,
}

/// Struct for keeping profile information
#[derive(Debug, Deserialize)]
struct Profile {
    application_id: String,
    api_key: String,
    default: Option<bool>,
}

impl Credentials {
    /// Resolve credentials with fallback order (first ENV, then default profile)
    pub fn load() -> Result<Self> {
        let profiles = load_profiles()?;
        let default = default_profile(&profiles).ok();

        let app_id = std::env::var("ALGOLIA_APPLICATION_ID")
            .ok()
            .or_else(|| default.as_ref().map(|p| p.application_id.clone()))
            .context("No application ID found")?;
        let api_key = std::env::var("ALGOLIA_API_KEY")
            .ok()
            .or_else(|| default.as_ref().map(|p| p.api_key.clone()))
            .context("No API key found")?;

        Ok(Self { app_id, api_key })
    }
}

/// Resolve the path to the config profile
fn profile_path() -> Result<PathBuf> {
    let dir = dirs::config_local_dir().context("Can't get config directory")?;

    Ok(dir.join("algolia").join("config.toml"))
}

/// Load and parse profiles
fn load_profiles() -> Result<HashMap<String, Profile>> {
    let path = profile_path()?;
    let content = std::fs::read_to_string(&path).context("Can't read profile file")?;
    let profiles: HashMap<String, Profile> =
        toml::from_str(&content).context("Can't parse config file")?;
    Ok(profiles)
}

/// Get the default profile
fn default_profile(profiles: &HashMap<String, Profile>) -> Result<&Profile> {
    profiles
        .values()
        .find(|profile| profile.default.unwrap_or(false))
        .context("No default profile found")
}

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

/// Candidate profile paths
fn profile_paths() -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    // Prefer `XDG_CONFIG_HOME` even on macOS
    let xdg_config_dir = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")));

    if let Some(dir) = xdg_config_dir {
        paths.push(dir.join("algolia").join("config.toml"));
    }

    // Also support platform-native paths
    let native = dirs::config_local_dir()
        .context("Can't get configuration directory")?
        .join("algolia")
        .join("config.toml");

    if !paths.contains(&native) {
        paths.push(native);
    }

    Ok(paths)
}

/// Load and parse profiles
fn load_profiles() -> Result<HashMap<String, Profile>> {
    let paths = profile_paths()?;

    let path = paths
        .iter()
        .find(|path| path.exists())
        .cloned()
        .context(format!(
            "Can't find profile file in any config directory: {}",
            paths
                .iter()
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ")
        ))?;

    let content = std::fs::read_to_string(&path).context(format!(
        "Can't read profile file: {}",
        path.to_string_lossy()
    ))?;
    let profiles: HashMap<String, Profile> =
        toml::from_str(&content).context("Can't parse config file.")?;

    Ok(profiles)
}

/// Get the default profile
fn default_profile(profiles: &HashMap<String, Profile>) -> Result<&Profile> {
    profiles
        .values()
        .find(|profile| profile.default.unwrap_or(false))
        .context("No default profile found")
}

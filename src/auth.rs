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

const APP_ID: &str = "ALGOLIA_APPLICATION_ID";
const API_KEY: &str = "ALGOLIA_API_KEY";
const CFG_DIR: &str = "XDG_CONFIG_HOME";

impl Credentials {
    /// Resolve credentials with fallback order (first ENV, then default profile)
    pub fn load() -> Result<Self> {
        // Return early if both environment variables are set to support
        // running without profiles
        let env_app_id = std::env::var(APP_ID).ok();
        let env_api_key = std::env::var(API_KEY).ok();

        if let (Some(app_id), Some(api_key)) = (&env_app_id, &env_api_key) {
            return Ok(Self {
                app_id: app_id.clone(),
                api_key: api_key.clone(),
            });
        }

        let profiles = load_profiles()?;
        let default = default_profile(&profiles).ok();

        let app_id = env_app_id
            .or_else(|| default.as_ref().map(|p| p.application_id.clone()))
            .context("No application ID found")?;

        let api_key = env_api_key
            .or_else(|| default.as_ref().map(|p| p.api_key.clone()))
            .context("No API key found")?;

        Ok(Self { app_id, api_key })
    }
}

/// Candidate profile paths
fn profile_paths() -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    // Prefer `XDG_CONFIG_HOME` even on macOS
    let xdg_config_dir = std::env::var_os(CFG_DIR)
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

#[cfg(test)]
mod test {
    use super::*;
    use serial_test::serial;

    fn clear_env() {
        unsafe {
            std::env::remove_var(APP_ID);
            std::env::remove_var(API_KEY);
            std::env::remove_var(CFG_DIR);
        }
    }

    fn write_config(contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join("algolia");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.toml"), contents).unwrap();

        unsafe {
            std::env::set_var(CFG_DIR, dir.path());
        }

        dir
    }

    #[serial]
    #[test]
    fn credentials_load_from_env_without_config_file() {
        unsafe {
            std::env::set_var(APP_ID, "test-app");
            std::env::set_var(API_KEY, "test-key");
            std::env::set_var(CFG_DIR, "/path/does/not/exist");
        }

        let creds = Credentials::load().unwrap();

        assert_eq!(creds.app_id, "test-app");
        assert_eq!(creds.api_key, "test-key");

        clear_env();
    }

    #[serial]
    #[test]
    fn load_credentials_from_default_profile() {
        clear_env();

        let dir = write_config(
            r#"
    [test]
    application_id = "test-config-app"
    api_key = "test-config-key"
    default = true
        "#,
        );

        let creds = Credentials::load().unwrap();

        assert_eq!(creds.app_id, "test-config-app");
        assert_eq!(creds.api_key, "test-config-key");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[serial]
    #[test]
    fn credentials_env_overrides_config() {
        clear_env();

        let dir = write_config(
            r#"
    [test]
    application_id = "test-config-app"
    api_key = "test-config-key"
    default = true
        "#,
        );

        unsafe {
            std::env::set_var(APP_ID, "test-env-app");
            std::env::set_var(API_KEY, "test-env-key");
        }

        let creds = Credentials::load().unwrap();

        assert_eq!(creds.app_id, "test-env-app");
        assert_eq!(creds.api_key, "test-env-key");

        std::fs::remove_dir_all(dir).unwrap();
        clear_env();
    }
}

//! GitHub Actions integration.
//!
//! When running inside GitHub Actions (`GITHUB_ACTIONS=true`), commands can
//! emit workflow commands (annotations) and append Markdown to the job summary.

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// GitHub Actions environment detected at startup
#[derive(Debug, Default, Clone)]
pub struct GitHubActions {
    enabled: bool,
    step_summary: Option<PathBuf>,
}

impl GitHubActions {
    /// Detect GitHub Actions from the environment
    #[must_use]
    pub fn from_env() -> Self {
        let enabled = std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true");
        let step_summary = std::env::var_os("GITHUB_STEP_SUMMARY")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);

        Self {
            enabled,
            step_summary,
        }
    }

    /// Construct explicitly (for tests)
    #[cfg(test)]
    pub fn new(enabled: bool, step_summary: Option<PathBuf>) -> Self {
        Self {
            enabled,
            step_summary,
        }
    }

    /// Format an `::error::` workflow command, or `None` outside GitHub Actions.
    ///
    /// Newlines and `%` are escaped as required by the workflow command syntax.
    #[must_use]
    pub fn error_annotation(&self, message: &str) -> Option<String> {
        self.enabled
            .then(|| format!("::error::{}", escape_data(message)))
    }

    /// Append Markdown to the job summary. No-op outside GitHub Actions.
    pub fn append_step_summary(&self, markdown: &str) -> Result<()> {
        let Some(path) = self.step_summary.as_ref().filter(|_| self.enabled) else {
            return Ok(());
        };

        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .with_context(|| format!("Can't open step summary {}", path.display()))?;

        writeln!(file, "{markdown}")?;
        Ok(())
    }
}

/// Escape message data for workflow commands
fn escape_data(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotation_is_none_when_disabled() {
        let gha = GitHubActions::new(false, None);
        assert_eq!(gha.error_annotation("boom"), None);
    }

    #[test]
    fn annotation_escapes_newlines_and_percent() {
        let gha = GitHubActions::new(true, None);
        assert_eq!(
            gha.error_annotation("100% broken\nsecond line").unwrap(),
            "::error::100%25 broken%0Asecond line"
        );
    }

    #[test]
    fn step_summary_appends_when_enabled() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let gha = GitHubActions::new(true, Some(file.path().to_path_buf()));

        gha.append_step_summary("## one").unwrap();
        gha.append_step_summary("two").unwrap();

        let content = std::fs::read_to_string(file.path()).unwrap();
        assert_eq!(content, "## one\ntwo\n");
    }

    #[test]
    fn step_summary_is_noop_when_disabled() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let gha = GitHubActions::new(false, Some(file.path().to_path_buf()));

        gha.append_step_summary("## one").unwrap();

        let content = std::fs::read_to_string(file.path()).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn step_summary_is_noop_without_path() {
        let gha = GitHubActions::new(true, None);
        gha.append_step_summary("## one").unwrap();
    }
}

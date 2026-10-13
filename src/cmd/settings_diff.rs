use crate::{
    api::SearchClient,
    cmd::{file_or_stdin, input_name},
    github::GitHubActions,
    output::OutputFormat,
};
use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

/// One differing top-level setting
#[derive(Debug, Serialize, PartialEq)]
pub struct Change {
    /// Setting name
    pub key: String,
    /// Value in the local file (`None` if the file doesn't set it)
    pub local: Option<Value>,
    /// Value in the remote index (`None` if the index doesn't have it)
    pub remote: Option<Value>,
}

/// Result of comparing local and remote settings
#[derive(Debug, Serialize, PartialEq)]
pub struct SettingsDiff {
    /// Index name
    pub index: String,
    /// Differences, sorted by key
    pub changes: Vec<Change>,
}

impl SettingsDiff {
    /// Compare two settings objects key by key.
    ///
    /// Only top-level keys are compared, nested values are compared wholesale.
    /// A key present in both with equal values is not a change.
    #[must_use]
    pub fn compute(index: &str, local: &Value, remote: &Value) -> Self {
        let empty = serde_json::Map::new();
        let local = local.as_object().unwrap_or(&empty);
        let remote = remote.as_object().unwrap_or(&empty);

        let keys: BTreeSet<&String> = local.keys().chain(remote.keys()).collect();

        let changes = keys
            .into_iter()
            .filter_map(|key| {
                let l = local.get(key);
                let r = remote.get(key);
                (l != r).then(|| Change {
                    key: key.clone(),
                    local: l.cloned(),
                    remote: r.cloned(),
                })
            })
            .collect();

        Self {
            index: index.to_string(),
            changes,
        }
    }

    /// Whether the settings match
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Plain-text rendering without colors (used for the job summary)
    #[must_use]
    pub fn to_plain_text(&self) -> String {
        let mut out = String::new();
        for change in &self.changes {
            match (&change.local, &change.remote) {
                (Some(l), Some(r)) => {
                    let _ = writeln!(out, "~ {}", change.key);
                    let _ = write!(out, "{}", indent_lines(&pretty(l), "  - "));
                    let _ = write!(out, "{}", indent_lines(&pretty(r), "  + "));
                }
                (Some(l), None) => {
                    let _ = writeln!(out, "- {} (only in file)", change.key);
                    let _ = write!(out, "{}", indent_lines(&pretty(l), "  - "));
                }
                (None, Some(r)) => {
                    let _ = writeln!(out, "+ {} (only in index)", change.key);
                    let _ = write!(out, "{}", indent_lines(&pretty(r), "  + "));
                }
                (None, None) => {}
            }
        }
        out
    }

    /// Colored rendering for terminals and CI logs
    #[must_use]
    pub fn to_colored_text(&self) -> String {
        let mut out = String::new();
        for change in &self.changes {
            match (&change.local, &change.remote) {
                (Some(l), Some(r)) => {
                    let _ = writeln!(out, "{}", format!("~ {}", change.key).yellow().bold());
                    let _ = write!(out, "{}", indent_lines(&pretty(l), "  - ").red());
                    let _ = write!(out, "{}", indent_lines(&pretty(r), "  + ").green());
                }
                (Some(l), None) => {
                    let _ = writeln!(
                        out,
                        "{}",
                        format!("- {} (only in file)", change.key).red().bold()
                    );
                    let _ = write!(out, "{}", indent_lines(&pretty(l), "  - ").red());
                }
                (None, Some(r)) => {
                    let _ = writeln!(
                        out,
                        "{}",
                        format!("+ {} (only in index)", change.key).green().bold()
                    );
                    let _ = write!(out, "{}", indent_lines(&pretty(r), "  + ").green());
                }
                (None, None) => {}
            }
        }
        out
    }

    /// Markdown for the GitHub job summary
    #[must_use]
    pub fn to_markdown(&self, file_name: &str) -> String {
        let n = self.changes.len();
        let noun = if n == 1 { "setting" } else { "settings" };
        format!(
            "### Settings drift in `{}`\n\n{n} {noun} differ between `{file_name}` (`-`) and the index (`+`).\n\n```diff\n{}```\n",
            self.index,
            self.to_plain_text()
        )
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn indent_lines(text: &str, prefix: &str) -> String {
    text.lines().fold(String::new(), |mut out, line| {
        let _ = writeln!(out, "{prefix}{line}");
        out
    })
}

/// Compare a local settings file with the settings of an index.
///
/// Returns the diff. The caller decides the exit code.
pub async fn diff_settings<W: std::io::Write>(
    client: &SearchClient,
    index_name: &str,
    file: Option<&Path>,
    output: OutputFormat,
    github: &GitHubActions,
    mut writer: W,
) -> Result<SettingsDiff> {
    let file_name = input_name(file);
    let input = file_or_stdin(file)?;
    let local: Value = serde_json::from_reader(input)
        .with_context(|| format!("Can't parse settings file {file_name}"))?;
    let remote = client.get_settings(index_name).await?;

    let diff = SettingsDiff::compute(index_name, &local, &remote);

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string(&diff).context("Can't serialize settings diff")?;
            writeln!(writer, "{json}")?;
        }
        OutputFormat::Text => {
            if diff.is_empty() {
                writeln!(
                    writer,
                    "{} Settings of {} match {}",
                    "🗸".green(),
                    index_name.bold(),
                    file_name.bold()
                )?;
            } else {
                write!(writer, "{}", diff.to_colored_text())?;
                writeln!(
                    writer,
                    "{} {} settings of {} differ from {}",
                    "✗".red(),
                    diff.changes.len(),
                    index_name.bold(),
                    file_name.bold()
                )?;
            }
        }
    }

    if !diff.is_empty() {
        let keys: Vec<&str> = diff.changes.iter().map(|c| c.key.as_str()).collect();
        let message = format!(
            "Settings drift in {index_name}: {} differ ({})",
            diff.changes.len(),
            keys.join(", ")
        );
        if let Some(annotation) = github.error_annotation(&message) {
            writeln!(writer, "{annotation}")?;
        }
        github.append_step_summary(&diff.to_markdown(&file_name))?;
    }

    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::test_support::AlgoliaMock;
    use httpmock::Method::GET;
    use serde_json::json;
    use std::io::Write;

    fn settings_file(value: &Value) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "{value}").unwrap();
        file
    }

    #[test]
    fn compute_reports_no_changes_for_equal_settings() {
        let a = json!({"distinct": true, "hitsPerPage": 20});
        let b = json!({"hitsPerPage": 20, "distinct": true});
        let diff = SettingsDiff::compute("idx", &a, &b);
        assert!(diff.is_empty());
    }

    #[test]
    fn compute_reports_changed_missing_and_extra_keys_sorted() {
        let local = json!({"distinct": true, "hitsPerPage": 20, "onlyLocal": 1});
        let remote = json!({"distinct": false, "hitsPerPage": 20, "aOnlyRemote": [1]});
        let diff = SettingsDiff::compute("idx", &local, &remote);

        assert_eq!(
            diff.changes,
            vec![
                Change {
                    key: "aOnlyRemote".into(),
                    local: None,
                    remote: Some(json!([1])),
                },
                Change {
                    key: "distinct".into(),
                    local: Some(json!(true)),
                    remote: Some(json!(false)),
                },
                Change {
                    key: "onlyLocal".into(),
                    local: Some(json!(1)),
                    remote: None,
                },
            ]
        );
    }

    #[test]
    fn compute_compares_nested_values_wholesale() {
        let local = json!({"ranking": ["words", "typo"]});
        let remote = json!({"ranking": ["typo", "words"]});
        let diff = SettingsDiff::compute("idx", &local, &remote);
        assert_eq!(diff.changes.len(), 1);
        assert_eq!(diff.changes[0].key, "ranking");
    }

    #[test]
    fn plain_text_rendering() {
        let local = json!({"distinct": true, "onlyLocal": "x"});
        let remote = json!({"distinct": false, "onlyRemote": [1, 2]});
        let diff = SettingsDiff::compute("idx", &local, &remote);

        let expected = "\
~ distinct
  - true
  + false
- onlyLocal (only in file)
  - \"x\"
+ onlyRemote (only in index)
  + [
  +   1,
  +   2
  + ]
";
        assert_eq!(diff.to_plain_text(), expected);
    }

    #[test]
    fn markdown_wraps_plain_text_in_diff_fence() {
        let diff = SettingsDiff::compute("idx", &json!({"a": 1}), &json!({"a": 2}));
        let md = diff.to_markdown("settings.json");
        assert!(md.starts_with("### Settings drift in `idx`"));
        assert!(md.contains("1 setting differ"));
        assert!(md.contains("```diff\n~ a\n  - 1\n  + 2\n```"));
    }

    #[tokio::test]
    async fn diff_settings_matching_prints_success() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(
            GET,
            "/1/indexes/idx/settings",
            200,
            json!({"distinct": true}),
        );
        let file = settings_file(&json!({"distinct": true}));
        let mut out = Vec::new();

        let diff = diff_settings(
            &api.client,
            "idx",
            Some(file.path()),
            OutputFormat::Text,
            &GitHubActions::new(false, None),
            &mut out,
        )
        .await
        .unwrap();

        mock.assert();
        assert!(diff.is_empty());
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Settings of idx match"));
        assert!(!text.contains("::error::"));
    }

    #[tokio::test]
    async fn diff_settings_with_drift_prints_diff_and_annotation() {
        let api = AlgoliaMock::new();
        api.json_response(
            GET,
            "/1/indexes/idx/settings",
            200,
            json!({"distinct": false, "hitsPerPage": 20}),
        );
        let file = settings_file(&json!({"distinct": true, "hitsPerPage": 20}));
        let summary = tempfile::NamedTempFile::new().unwrap();
        let mut out = Vec::new();

        let diff = diff_settings(
            &api.client,
            "idx",
            Some(file.path()),
            OutputFormat::Text,
            &GitHubActions::new(true, Some(summary.path().to_path_buf())),
            &mut out,
        )
        .await
        .unwrap();

        assert_eq!(diff.changes.len(), 1);
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("~ distinct"));
        assert!(text.contains("1 settings of idx differ"));
        assert!(text.contains("::error::Settings drift in idx: 1 differ (distinct)"));

        let summary_text = std::fs::read_to_string(summary.path()).unwrap();
        assert!(summary_text.contains("### Settings drift in `idx`"));
        assert!(summary_text.contains("~ distinct"));
    }

    #[tokio::test]
    async fn diff_settings_json_output() {
        let api = AlgoliaMock::new();
        api.json_response(GET, "/1/indexes/idx/settings", 200, json!({"a": 2}));
        let file = settings_file(&json!({"a": 1}));
        let mut out = Vec::new();

        diff_settings(
            &api.client,
            "idx",
            Some(file.path()),
            OutputFormat::Json,
            &GitHubActions::new(false, None),
            &mut out,
        )
        .await
        .unwrap();

        let parsed: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(
            parsed,
            json!({"index": "idx", "changes": [{"key": "a", "local": 1, "remote": 2}]})
        );
    }

    #[tokio::test]
    async fn diff_settings_fails_on_invalid_file() {
        let api = AlgoliaMock::new();
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "not json").unwrap();

        let err = diff_settings(
            &api.client,
            "idx",
            Some(file.path()),
            OutputFormat::Text,
            &GitHubActions::new(false, None),
            Vec::new(),
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("Can't parse settings file"));
    }
}

use crate::{
    api::SearchClient,
    cmd::{file_or_stdin, input_name},
    output::OutputFormat,
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

/// Apply a settings file to an index and wait until the settings are published.
pub async fn import_settings<W: std::io::Write>(
    client: &SearchClient,
    index_name: &str,
    file: Option<&Path>,
    output: OutputFormat,
    mut writer: W,
) -> Result<()> {
    let name = input_name(file);
    let input = file_or_stdin(file)?;
    let settings: serde_json::Value = serde_json::from_reader(input)
        .with_context(|| format!("Can't parse settings file {name}"))?;

    let response = client.set_settings(index_name, &settings).await?;
    client.wait_for_task(index_name, response.task_id).await?;

    match output {
        OutputFormat::Json => {
            let json =
                serde_json::to_string(&response).context("Can't serialize settings response")?;
            writeln!(writer, "{json}")?;
        }
        OutputFormat::Text => {
            writeln!(
                writer,
                "{} Updated settings of {} from {} (task {})",
                "🗸".green(),
                index_name.bold(),
                name.bold(),
                response.task_id.to_string().dimmed()
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::test_support::AlgoliaMock;
    use httpmock::Method::{GET, PUT};
    use serde_json::json;
    use std::io::Write;

    fn settings_file() -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, r#"{{"distinct": true}}"#).unwrap();
        file
    }

    #[tokio::test]
    async fn import_puts_settings_and_waits_for_task() {
        let api = AlgoliaMock::new();
        let put = api.server.mock(|when, then| {
            when.method(PUT)
                .path("/1/indexes/idx/settings")
                .json_body(json!({"distinct": true}));
            then.status(200)
                .json_body(json!({"taskID": 7, "updatedAt": "2026-05-20T19:35"}));
        });
        let task = api.json_response(
            GET,
            "/1/indexes/idx/task/7",
            200,
            json!({"status": "published"}),
        );
        let file = settings_file();
        let mut out = Vec::new();

        import_settings(
            &api.client,
            "idx",
            Some(file.path()),
            OutputFormat::Text,
            &mut out,
        )
        .await
        .unwrap();

        put.assert();
        task.assert();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Updated settings of idx"));
        assert!(text.contains("task 7"));
    }

    #[tokio::test]
    async fn import_json_output() {
        let api = AlgoliaMock::new();
        api.server.mock(|when, then| {
            when.method(PUT).path("/1/indexes/idx/settings");
            then.status(200)
                .json_body(json!({"taskID": 7, "updatedAt": "2026-05-20T19:35"}));
        });
        api.json_response(
            GET,
            "/1/indexes/idx/task/7",
            200,
            json!({"status": "published"}),
        );
        let file = settings_file();
        let mut out = Vec::new();

        import_settings(
            &api.client,
            "idx",
            Some(file.path()),
            OutputFormat::Json,
            &mut out,
        )
        .await
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(
            parsed,
            json!({"taskID": 7, "updatedAt": "2026-05-20T19:35"})
        );
    }

    #[tokio::test]
    async fn import_fails_on_invalid_json() {
        let api = AlgoliaMock::new();
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "nope").unwrap();

        let err = import_settings(
            &api.client,
            "idx",
            Some(file.path()),
            OutputFormat::Text,
            Vec::new(),
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("Can't parse settings file"));
    }
}

use crate::{api::SearchClient, output::OutputFormat};
use anyhow::{Context, Result};

/// Print the settings of an index as JSON.
///
/// Text output is pretty-printed, JSON output is one line.
pub async fn get_settings<W: std::io::Write>(
    client: &SearchClient,
    index_name: &str,
    output: OutputFormat,
    mut writer: W,
) -> Result<()> {
    let settings = client.get_settings(index_name).await?;

    let json = match output {
        OutputFormat::Json => serde_json::to_string(&settings),
        OutputFormat::Text => serde_json::to_string_pretty(&settings),
    }
    .context("Can't serialize settings")?;

    writeln!(writer, "{json}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::test_support::AlgoliaMock;
    use httpmock::Method::GET;
    use serde_json::json;

    #[tokio::test]
    async fn text_output_is_pretty_printed() {
        let api = AlgoliaMock::new();
        api.json_response(
            GET,
            "/1/indexes/idx/settings",
            200,
            json!({"distinct": true, "hitsPerPage": 20}),
        );
        let mut out = Vec::new();

        get_settings(&api.client, "idx", OutputFormat::Text, &mut out)
            .await
            .unwrap();

        assert_eq!(
            String::from_utf8(out).unwrap(),
            "{\n  \"distinct\": true,\n  \"hitsPerPage\": 20\n}\n"
        );
    }

    #[tokio::test]
    async fn json_output_is_one_line() {
        let api = AlgoliaMock::new();
        api.json_response(
            GET,
            "/1/indexes/idx/settings",
            200,
            json!({"distinct": true, "hitsPerPage": 20}),
        );
        let mut out = Vec::new();

        get_settings(&api.client, "idx", OutputFormat::Json, &mut out)
            .await
            .unwrap();

        assert_eq!(
            String::from_utf8(out).unwrap(),
            "{\"distinct\":true,\"hitsPerPage\":20}\n"
        );
    }
}

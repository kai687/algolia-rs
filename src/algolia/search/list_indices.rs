use super::SearchClient;
use anyhow::Result;
use bytesize::ByteSize;
use chrono::DateTime;
use num_format::{Locale, ToFormattedString};
use serde::{Deserialize, Serialize};
use tabled::Tabled;
use timeago::Formatter;

/// Format large numbers
fn format_large_numbers<T: ToFormattedString>(val: &T) -> String {
    val.to_formatted_string(&Locale::en)
}

/// Format file sizes
fn format_sizes(val: &u64) -> String {
    ByteSize(*val).display().si().to_string()
}

/// Format timestamps relative to current time
fn format_relative_datetime(val: &str) -> String {
    let Ok(dt) = DateTime::parse_from_rfc3339(val) else {
        return val.to_string();
    };
    let duration = chrono::Utc::now()
        .signed_duration_since(dt)
        .to_std()
        .unwrap_or_default();
    Formatter::new().convert(duration)
}

/// The response for the 'list indices' endpoint
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListIndicesResponse {
    pub items: Vec<ListIndexItem>,
    pub nb_pages: u32,
}

#[derive(Debug, Serialize, Deserialize, Tabled)]
#[serde(rename_all = "camelCase")]
#[tabled(rename_all = "UPPERCASE")]
pub struct ListIndexItem {
    /// Index name
    pub name: String,
    /// Number of records in this index
    #[tabled(rename = "RECORDS", display = "format_large_numbers")]
    pub entries: u32,
    /// Size of the minified index in bytes
    #[tabled(rename = "SIZE", display = "format_sizes")]
    pub data_size: u64,
    /// Date and time when the index was created
    #[tabled(rename = "CREATED", display = "format_relative_datetime")]
    pub created_at: String,
    /// Date and time when the index was last updated
    #[tabled(rename = "UPDATED", display = "format_relative_datetime")]
    pub updated_at: String,
    /// Size of the index file on disk in bytes
    #[tabled(skip)]
    pub file_size: u64,
    /// How long it took to build the index in seconds
    #[tabled(skip)]
    pub last_build_time_s: u32,
    /// Number of pending tasks for this index
    #[tabled(skip)]
    pub number_of_pending_tasks: u32,
    /// (Replica indices only) Name of the primary index
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tabled(skip)]
    pub primary: Option<String>,
    /// (Primary indices with replicas only) Replica index names
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tabled(skip)]
    pub replicas: Option<Vec<String>>,
    /// Whether this index has pending tasks
    #[tabled(skip)]
    pub pending_task: bool,
    /// Virtual replica indices only (virtual is a reserved keyword)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tabled(skip)]
    pub r#virtual: Option<bool>,
    /// Index type: replica, primary, virtual replica. Table-only property
    #[serde(skip)]
    #[tabled(rename = "INDEX TYPE")]
    pub index_type: String,
}

impl ListIndexItem {
    /// Compute index type based on other properties
    pub fn with_index_type(mut self) -> Self {
        self.index_type = match (&self.primary, &self.replicas, &self.r#virtual) {
            (Some(p), _, Some(true)) => format!("virtual replica of {p}"),
            (Some(p), _, _) => format!("replica of {p}"),
            (_, Some(r), _) => {
                let n = r.len();
                let mut word = "replica";
                if n > 1 {
                    word = "replicas";
                }
                format!("primary ({} {})", n, word)
            }
            _ => "".to_string(),
        };

        self
    }
}

impl SearchClient {
    /// List all indices
    pub async fn list_indices(&self) -> Result<ListIndicesResponse> {
        self.get("/indexes").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    #[tokio::test]
    async fn list_indices_request() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/1/indexes");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    serde_json::json!({
                        "items": [
                            {
                                "name": "Test",
                                "entries": 0,
                                "createdAt": "2026-04-29T08:50:12.848Z",
                                "updatedAt": "2026-04-29T08:50:12.848Z",
                                "dataSize": 1,
                                "fileSize": 1,
                                "lastBuildTimeS": 1,
                                "numberOfPendingTasks": 0,
                                "pendingTask": false
                            }
                        ],
                        "nbPages": 1
                    })
                    .to_string(),
                );
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        client.list_indices().await.unwrap();

        mock.assert();
    }
}

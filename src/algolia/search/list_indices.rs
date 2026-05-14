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
#[allow(clippy::trivially_copy_pass_by_ref)]
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
#[must_use]
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
                format!("primary ({n} {word})")
            }
            _ => String::new(),
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
                .json_body(serde_json::json!({
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
                }));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let response = client.list_indices().await.unwrap();

        mock.assert();
        assert_eq!(response.nb_pages, 1);
        assert_eq!(response.items.len(), 1);
        let item = response.items.first().unwrap();
        assert_eq!(item.name, "Test");
        assert_eq!(item.entries, 0);
        assert_eq!(item.data_size, 1);
        assert!(!item.pending_task);
    }

    /// Test helper for `with_index_type` tests
    fn item() -> ListIndexItem {
        ListIndexItem {
            name: "test-index".to_string(),
            entries: 0,
            data_size: 0,
            created_at: "2026-04-29T08:50:12.848Z".to_string(),
            updated_at: "2026-04-29T08:50:12.848Z".to_string(),
            file_size: 0,
            last_build_time_s: 0,
            number_of_pending_tasks: 0,
            primary: None,
            replicas: None,
            pending_task: false,
            r#virtual: None,
            index_type: String::new(),
        }
    }

    #[test]
    fn item_for_regular_index() {
        let item = item().with_index_type();
        assert_eq!(item.index_type, "");
    }

    #[test]
    fn item_for_primary_index_with_one_replica() {
        let item = ListIndexItem {
            replicas: Some(vec!["replica_index".to_string()]),
            ..item()
        }
        .with_index_type();

        assert_eq!(item.index_type, "primary (1 replica)");
    }

    #[test]
    fn item_for_primary_index_with_two_replicas() {
        let item = ListIndexItem {
            replicas: Some(vec!["replica_1".to_string(), "replica_2".to_string()]),
            ..item()
        }
        .with_index_type();

        assert_eq!(item.index_type, "primary (2 replicas)");
    }

    #[test]
    fn item_for_replica_index() {
        let item = ListIndexItem {
            primary: Some("primary-index".to_string()),
            ..item()
        }
        .with_index_type();

        assert_eq!(item.index_type, "replica of primary-index");
    }

    #[test]
    fn item_for_virtual_replica() {
        let item = ListIndexItem {
            primary: Some("primary-index".to_string()),
            r#virtual: Some(true),
            ..item()
        }
        .with_index_type();

        assert_eq!(item.index_type, "virtual replica of primary-index");
    }
}

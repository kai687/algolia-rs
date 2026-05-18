use super::{RecordWithObjectID, SearchClient};
use anyhow::Result;
use serde::Deserialize;

/// We only care about the `hits` field
#[derive(Deserialize)]
struct BrowseResponse {
    hits: Vec<RecordWithObjectID>,
    cursor: Option<String>,
}

impl SearchClient {
    /// Browse for objects endpoint
    async fn browse(
        self: &SearchClient,
        index_name: &str,
        _cursor: Option<String>,
    ) -> Result<BrowseResponse> {
        let request_url = format!("/indexes/{index_name}/browse");
        self.post::<BrowseResponse, serde_json::Value>(&request_url, None)
            .await
    }

    /// Wrapper around `browse` to get all records from paginated response
    pub async fn list_records(
        self: &SearchClient,
        index_name: &str,
    ) -> Result<Vec<RecordWithObjectID>> {
        let mut records: Vec<RecordWithObjectID> = vec![];
        let response = self.browse(index_name, None).await?;
        records.extend(response.hits);

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use serde_json::json;

    #[tokio::test]
    async fn send_browse_request() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/1/indexes/test-index/browse");
            then.status(200).json_body(
                json!({"hits": [{"objectID": "1", "name": "foo"}], "cursor": "test-cursor"}),
            );
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let response = client.browse("test-index", None).await.unwrap();

        mock.assert();

        assert_eq!(response.hits.len(), 1);
        assert_eq!(response.cursor.unwrap(), "test-cursor");
    }

    #[tokio::test]
    async fn send_list_records_request() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/1/indexes/test-index/browse");
            then.status(200)
                .json_body(json!({"hits": [{"objectID": "1", "name": "foo"}]}));
        });

        let client = SearchClient::new_with_base_url(
            "test-app",
            "test-key",
            format!("{}/1", server.base_url()),
        )
        .unwrap();

        let records = client.list_records("test-index").await.unwrap();

        mock.assert();

        assert_eq!(records.len(), 1);
    }
}

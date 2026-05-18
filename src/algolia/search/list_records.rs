use super::{RecordWithObjectID, SearchClient};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// We only care about the `hits` field
#[derive(Deserialize)]
struct BrowseResponse {
    hits: Vec<RecordWithObjectID>,
    cursor: Option<String>,
}

#[derive(Serialize)]
struct BrowseRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
}

impl SearchClient {
    /// Browse for objects endpoint
    async fn browse(
        self: &SearchClient,
        index_name: &str,
        cursor: Option<String>,
    ) -> Result<BrowseResponse> {
        let request_url = format!("/indexes/{index_name}/browse");
        // browse request needs empty `{}` body, not `null`
        let data = BrowseRequest { cursor };
        self.post::<BrowseResponse, BrowseRequest>(&request_url, Some(&data))
            .await
    }

    /// Browse records page-by-page, calling `on_page` as soon as each page arrives.
    pub async fn for_each_record_page<F>(&self, index_name: &str, mut on_page: F) -> Result<()>
    where
        F: FnMut(Vec<RecordWithObjectID>) -> Result<()>,
    {
        let mut cursor = None;

        // Call the endpoint repeatedly until the response doesn't include a `cursor` field.
        loop {
            let response = self.browse(index_name, cursor).await?;
            on_page(response.hits)?;

            cursor = response.cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(())
    }

    /// Wrapper around `browse` to get all records from paginated response.
    pub async fn list_records(&self, index_name: &str) -> Result<Vec<RecordWithObjectID>> {
        let mut records: Vec<RecordWithObjectID> = vec![];

        self.for_each_record_page(index_name, |page| {
            records.extend(page);
            Ok(())
        })
        .await?;

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

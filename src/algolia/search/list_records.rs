use super::SearchClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Browse response that keeps each hit as raw JSON.
#[derive(Deserialize)]
struct BrowseResponse {
    hits: Vec<Box<RawValue>>,
    cursor: Option<String>,
}

#[derive(Serialize)]
struct BrowseRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
}

impl SearchClient {
    /// Iterate over paginated browse response, streaming results as they come in,
    /// skipping deserialization as we only print out raw JSON records in the end
    pub async fn list_records<F>(&self, index_name: &str, mut on_page: F) -> Result<()>
    where
        F: FnMut(Vec<Box<RawValue>>) -> Result<()>,
    {
        let request_url = format!("/indexes/{index_name}/browse");
        let mut cursor = None;

        // Call the endpoint repeatedly until the response doesn't include a `cursor` field.
        loop {
            let data = BrowseRequest { cursor };
            let response = self
                .post::<BrowseResponse, BrowseRequest>(&request_url, &data)
                .await?;
            on_page(response.hits)?;

            cursor = response.cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::algolia::search::test_support::AlgoliaMock;
    use httpmock::Method::POST;
    use serde_json::json;

    #[tokio::test]
    async fn list_records_succeeds() {
        let api = AlgoliaMock::new();

        let first_page = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/browse")
                .json_body(json!({}));

            then.status(200)
                .header("content-type", "application/json")
                .json_body(
                    json!({"hits": [{"objectID": "1", "name": "foo"}], "cursor": "next-page"}),
                );
        });

        let second_page = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/browse")
                .json_body(json!({"cursor": "next-page"}));

            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"hits": [{"objectID": "2", "name": "bar"}]}));
        });

        let mut records = Vec::new();
        api.client
            .list_records("test-index", |page| {
                records.extend(page.into_iter().map(|record| record.get().to_string()));
                Ok(())
            })
            .await
            .unwrap();

        first_page.assert();
        second_page.assert();

        assert_eq!(records.len(), 2);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&records[0]).unwrap(),
            json!({"objectID": "1", "name": "foo"})
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&records[1]).unwrap(),
            json!({"objectID": "2", "name": "bar"})
        );
    }
}

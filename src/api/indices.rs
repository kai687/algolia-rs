use super::{SearchClient, UpdatedAtResponse};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// What to include when copying an index
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    Settings,
    Synonyms,
    Rules,
}

/// Request body for copying or moving an index
#[derive(Serialize)]
#[serde(tag = "operation", rename_all = "camelCase")]
enum IndexOperation<'a> {
    Copy {
        destination: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<&'a [Scope]>,
    },
    Move {
        destination: &'a str,
    },
}

/// Response of the delete index endpoint
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeletedAtResponse {
    #[serde(rename = "taskID")]
    pub task_id: u64,
    pub deleted_at: String,
}

impl SearchClient {
    /// Copy an index to `destination`, overwriting it.
    ///
    /// With `scope`, only the listed parts are copied (no records).
    pub async fn copy_index(
        &self,
        index_name: &str,
        destination: &str,
        scope: Option<&[Scope]>,
    ) -> Result<UpdatedAtResponse> {
        let operation = IndexOperation::Copy { destination, scope };
        self.post(&format!("/indexes/{index_name}/operation"), &operation)
            .await
    }

    /// Move (rename) an index to `destination`, overwriting it.
    pub async fn move_index(
        &self,
        index_name: &str,
        destination: &str,
    ) -> Result<UpdatedAtResponse> {
        let operation = IndexOperation::Move { destination };
        self.post(&format!("/indexes/{index_name}/operation"), &operation)
            .await
    }

    /// Delete an index
    pub async fn delete_index(&self, index_name: &str) -> Result<DeletedAtResponse> {
        self.delete(&format!("/indexes/{index_name}")).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::test_support::AlgoliaMock;
    use httpmock::Method::{DELETE, POST};
    use serde_json::json;

    #[test]
    fn copy_body_without_scope_omits_scope() {
        let op = IndexOperation::Copy {
            destination: "new_index",
            scope: None,
        };
        assert_eq!(
            serde_json::to_value(&op).unwrap(),
            json!({"operation": "copy", "destination": "new_index"})
        );
    }

    #[test]
    fn copy_body_with_scope() {
        let op = IndexOperation::Copy {
            destination: "new_index",
            scope: Some(&[Scope::Settings, Scope::Synonyms]),
        };
        assert_eq!(
            serde_json::to_value(&op).unwrap(),
            json!({"operation": "copy", "destination": "new_index", "scope": ["settings", "synonyms"]})
        );
    }

    #[test]
    fn move_body() {
        let op = IndexOperation::Move {
            destination: "new_index",
        };
        assert_eq!(
            serde_json::to_value(&op).unwrap(),
            json!({"operation": "move", "destination": "new_index"})
        );
    }

    #[tokio::test]
    async fn copy_index_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/operation")
                .json_body(json!({"operation": "copy", "destination": "dest"}));
            then.status(200)
                .json_body(json!({"taskID": 123_456, "updatedAt": "2026-04-29T08:50:12.848Z"}));
        });

        let response = api
            .client
            .copy_index("test-index", "dest", None)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(response.task_id, 123_456);
        assert_eq!(response.updated_at, "2026-04-29T08:50:12.848Z");
    }

    #[tokio::test]
    async fn scoped_copy_index_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/operation")
                .json_body(json!({"operation": "copy", "destination": "dest", "scope": ["settings", "synonyms", "rules"]}));
            then.status(200)
                .json_body(json!({"taskID": 1, "updatedAt": "2026-04-29T08:50:12.848Z"}));
        });

        api.client
            .copy_index(
                "test-index",
                "dest",
                Some(&[Scope::Settings, Scope::Synonyms, Scope::Rules]),
            )
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn move_index_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.server.mock(|when, then| {
            when.method(POST)
                .path("/1/indexes/test-index/operation")
                .json_body(json!({"operation": "move", "destination": "dest"}));
            then.status(200)
                .json_body(json!({"taskID": 123_456, "updatedAt": "2026-04-29T08:50:12.848Z"}));
        });

        let response = api.client.move_index("test-index", "dest").await.unwrap();

        mock.assert();
        assert_eq!(response.task_id, 123_456);
    }

    #[tokio::test]
    async fn delete_index_succeeds() {
        let api = AlgoliaMock::new();
        let mock = api.json_response(
            DELETE,
            "/1/indexes/test-index",
            200,
            json!({"taskID": 123_456, "deletedAt": "2026-05-14T19:44"}),
        );

        let response = api.client.delete_index("test-index").await.unwrap();

        mock.assert();
        assert_eq!(response.task_id, 123_456);
        assert_eq!(response.deleted_at, "2026-05-14T19:44");
    }
}

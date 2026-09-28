mod common;

use axum::http::StatusCode;
use common::{MAX_UPLOAD_BYTES, TestApp};
use serde_json::json;
use trovr_storage::ObjectStorage;

#[tokio::test]
async fn upload_confirm_and_download_round_trip() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;
    let folder = app
        .post("/api/v1/folders", &alice.token, json!({ "name": "docs" }))
        .await
        .body;

    let file = app
        .upload_file(&alice, folder["id"].as_str(), "hello.txt", b"hello")
        .await;
    assert_eq!(file["type"], "file");
    assert_eq!(file["size_bytes"], 5);
    assert_eq!(file["mime_type"], "text/plain");
    assert_eq!(file["parent_id"], folder["id"]);

    let file_id = file["id"].as_str().unwrap();
    let download = app
        .get(&format!("/api/v1/nodes/{file_id}/download"), &alice.token)
        .await;
    assert_eq!(download.status, StatusCode::OK, "{:?}", download.body);
    assert_eq!(download.body["method"], "GET");

    let response = app.fetch(&download.body).await;
    assert!(response.status().is_success());
    let disposition = response.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .to_string();
    assert!(disposition.contains("hello.txt"), "{disposition}");
    assert_eq!(response.bytes().await.unwrap().as_ref(), b"hello");
}

#[tokio::test]
async fn new_versions_replace_the_content_and_old_ones_stay_downloadable() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;
    let file = app.upload_file(&alice, None, "notes.txt", b"v1").await;
    let file_id = file["id"].as_str().unwrap();

    let storage_key = app.upload(&alice, "text/plain", b"version two").await;
    let version = app
        .post(
            &format!("/api/v1/nodes/{file_id}/versions"),
            &alice.token,
            json!({ "storage_key": storage_key }),
        )
        .await;
    assert_eq!(version.status, StatusCode::CREATED, "{:?}", version.body);
    assert_eq!(version.body["version_number"], 2);

    let versions = app
        .get(&format!("/api/v1/nodes/{file_id}/versions"), &alice.token)
        .await;
    let numbers: Vec<i64> = versions
        .body
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["version_number"].as_i64().unwrap())
        .collect();
    assert_eq!(numbers, vec![2, 1]);

    let node = app
        .get(&format!("/api/v1/nodes/{file_id}"), &alice.token)
        .await;
    assert_eq!(node.body["size_bytes"], 11);

    let current = app
        .get(&format!("/api/v1/nodes/{file_id}/download"), &alice.token)
        .await;
    let current_bytes = app.fetch(&current.body).await.bytes().await.unwrap();
    assert_eq!(current_bytes.as_ref(), b"version two");

    let first_id = versions.body[1]["id"].as_str().unwrap();
    let old = app
        .get(
            &format!("/api/v1/nodes/{file_id}/download?version_id={first_id}"),
            &alice.token,
        )
        .await;
    let old_bytes = app.fetch(&old.body).await.bytes().await.unwrap();
    assert_eq!(old_bytes.as_ref(), b"v1");
}

#[tokio::test]
async fn uploads_cannot_be_adopted_or_confirmed_twice() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let storage_key = app.upload(&alice, "text/plain", b"alice's data").await;

    let adopted = app
        .post(
            "/api/v1/files",
            &bob.token,
            json!({ "storage_key": storage_key, "name": "stolen.txt" }),
        )
        .await;
    assert_eq!(adopted.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(adopted.body["error"]["code"], "invalid_upload");

    let bobs_file = app.upload_file(&bob, None, "mine.txt", b"bob").await;
    let adopted_as_version = app
        .post(
            &format!(
                "/api/v1/nodes/{}/versions",
                bobs_file["id"].as_str().unwrap()
            ),
            &bob.token,
            json!({ "storage_key": storage_key }),
        )
        .await;
    assert_eq!(adopted_as_version.status, StatusCode::UNPROCESSABLE_ENTITY);

    let first = app
        .post(
            "/api/v1/files",
            &alice.token,
            json!({ "storage_key": storage_key, "name": "a.txt" }),
        )
        .await;
    assert_eq!(first.status, StatusCode::CREATED);

    let second = app
        .post(
            "/api/v1/files",
            &alice.token,
            json!({ "storage_key": storage_key, "name": "b.txt" }),
        )
        .await;
    assert_eq!(second.status, StatusCode::CONFLICT);
    assert_eq!(second.body["error"]["code"], "upload_already_used");
}

#[tokio::test]
async fn confirming_without_uploading_is_refused() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;
    let initiated = app
        .post(
            "/api/v1/uploads",
            &alice.token,
            json!({ "size_bytes": 3, "mime_type": "text/plain" }),
        )
        .await;

    let confirmed = app
        .post(
            "/api/v1/files",
            &alice.token,
            json!({ "storage_key": initiated.body["storage_key"], "name": "ghost.txt" }),
        )
        .await;

    assert_eq!(confirmed.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(confirmed.body["error"]["code"], "upload_missing");
}

#[tokio::test]
async fn oversized_uploads_are_refused_and_deleted() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;

    let declared_too_big = app
        .post(
            "/api/v1/uploads",
            &alice.token,
            json!({ "size_bytes": MAX_UPLOAD_BYTES + 1, "mime_type": "text/plain" }),
        )
        .await;
    assert_eq!(declared_too_big.status, StatusCode::PAYLOAD_TOO_LARGE);

    // Lies about its size: declares 10 bytes, PUTs more than the limit.
    let initiated = app
        .post(
            "/api/v1/uploads",
            &alice.token,
            json!({ "size_bytes": 10, "mime_type": "text/plain" }),
        )
        .await;
    let oversized = vec![b'x'; usize::try_from(MAX_UPLOAD_BYTES).unwrap() + 1];
    app.put_presigned(&initiated.body["upload"], &oversized)
        .await;
    let storage_key = initiated.body["storage_key"].as_str().unwrap();

    let confirmed = app
        .post(
            "/api/v1/files",
            &alice.token,
            json!({ "storage_key": storage_key, "name": "big.txt" }),
        )
        .await;

    assert_eq!(confirmed.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(app.state.storage.head(storage_key).await.unwrap(), None);
    assert!(
        app.get("/api/v1/nodes", &alice.token)
            .await
            .body
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn upload_requests_are_validated() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;

    for body in [
        json!({ "size_bytes": -1, "mime_type": "text/plain" }),
        json!({ "size_bytes": 1, "mime_type": "" }),
        json!({ "size_bytes": 1, "mime_type": "text/plain\r\nx-evil: 1" }),
    ] {
        let response = app
            .post("/api/v1/uploads", &alice.token, body.clone())
            .await;
        assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }

    let initiated = app
        .post(
            "/api/v1/uploads",
            &alice.token,
            json!({ "size_bytes": 0, "mime_type": "application/pdf" }),
        )
        .await;
    assert_eq!(initiated.status, StatusCode::CREATED);
    let storage_key = initiated.body["storage_key"].as_str().unwrap();
    assert!(ObjectStorage::key_belongs_to(storage_key, alice.id));
    assert_eq!(initiated.body["upload"]["method"], "PUT");
}

#[tokio::test]
async fn other_users_files_are_invisible() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let file = app
        .upload_file(&alice, None, "private.txt", b"secret")
        .await;
    let file_id = file["id"].as_str().unwrap();
    let bobs_key = app.upload(&bob, "text/plain", b"bob").await;

    let attempts = [
        app.get(&format!("/api/v1/nodes/{file_id}/download"), &bob.token)
            .await,
        app.get(&format!("/api/v1/nodes/{file_id}/versions"), &bob.token)
            .await,
        app.post(
            &format!("/api/v1/nodes/{file_id}/versions"),
            &bob.token,
            json!({ "storage_key": bobs_key }),
        )
        .await,
        app.post(
            "/api/v1/files",
            &bob.token,
            json!({ "storage_key": bobs_key, "parent_id": file_id, "name": "x.txt" }),
        )
        .await,
    ];
    for response in attempts {
        assert_eq!(
            response.status,
            StatusCode::NOT_FOUND,
            "{:?}",
            response.body
        );
    }
}

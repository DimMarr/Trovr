mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::{Value, json};

async fn create_folder(app: &TestApp, token: &str, parent_id: Option<&str>, name: &str) -> Value {
    app.post(
        "/api/v1/folders",
        token,
        json!({ "parent_id": parent_id, "name": name }),
    )
    .await
    .body
}

#[tokio::test]
async fn trash_then_restore_round_trip() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let folder = create_folder(&app, &alice.token, None, "folder").await;
    let folder_id = folder["id"].as_str().unwrap();
    create_folder(&app, &alice.token, Some(folder_id), "child").await;

    let trashed = app
        .delete(&format!("/api/v1/nodes/{folder_id}"), &alice.token)
        .await;
    assert_eq!(trashed.status, StatusCode::OK);
    assert!(trashed.body["trashed_at"].is_string());

    let gone = app
        .get(&format!("/api/v1/nodes/{folder_id}"), &alice.token)
        .await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);

    let trash = app.get("/api/v1/trash", &alice.token).await;
    assert_eq!(trash.body.as_array().unwrap().len(), 1);
    assert_eq!(trash.body[0]["id"], folder["id"]);

    let restored = app
        .post(
            &format!("/api/v1/nodes/{folder_id}/restore"),
            &alice.token,
            json!({}),
        )
        .await;
    assert_eq!(restored.status, StatusCode::OK);
    assert_eq!(restored.body["trashed_at"], Value::Null);

    let children = app
        .get(&format!("/api/v1/nodes/{folder_id}/children"), &alice.token)
        .await;
    assert_eq!(children.body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn restore_and_purge_require_a_trashed_node() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let folder = create_folder(&app, &alice.token, None, "folder").await;
    let folder_id = folder["id"].as_str().unwrap();

    let restore = app
        .post(
            &format!("/api/v1/nodes/{folder_id}/restore"),
            &alice.token,
            json!({}),
        )
        .await;
    let purge = app
        .delete(&format!("/api/v1/trash/{folder_id}"), &alice.token)
        .await;

    assert_eq!(restore.status, StatusCode::CONFLICT);
    assert_eq!(restore.body["error"]["code"], "not_trashed");
    assert_eq!(purge.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn other_users_trash_is_invisible() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let live = create_folder(&app, &alice.token, None, "live").await;
    let trashed = create_folder(&app, &alice.token, None, "trashed").await;
    let live_id = live["id"].as_str().unwrap();
    let trashed_id = trashed["id"].as_str().unwrap();
    app.delete(&format!("/api/v1/nodes/{trashed_id}"), &alice.token)
        .await;

    let attempts = [
        app.delete(&format!("/api/v1/nodes/{live_id}"), &bob.token)
            .await,
        app.post(
            &format!("/api/v1/nodes/{trashed_id}/restore"),
            &bob.token,
            json!({}),
        )
        .await,
        app.delete(&format!("/api/v1/trash/{trashed_id}"), &bob.token)
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

    assert!(
        app.get("/api/v1/trash", &bob.token)
            .await
            .body
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        app.get("/api/v1/trash", &alice.token)
            .await
            .body
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn purge_deletes_the_subtree_and_its_stored_objects() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;
    let folder = create_folder(&app, &alice.token, None, "folder").await;
    let folder_id = folder["id"].as_str().unwrap();
    let file = app
        .upload_file(&alice, Some(folder_id), "a.txt", b"one")
        .await;
    let file_id = file["id"].as_str().unwrap();
    let second_key = app.upload(&alice, "text/plain", b"two").await;
    app.post(
        &format!("/api/v1/nodes/{file_id}/versions"),
        &alice.token,
        json!({ "storage_key": second_key }),
    )
    .await;
    let keys: Vec<String> =
        sqlx::query_scalar("SELECT storage_key FROM file_versions ORDER BY version_number")
            .fetch_all(&app.state.pool)
            .await
            .unwrap();
    assert_eq!(keys.len(), 2);

    app.delete(&format!("/api/v1/nodes/{folder_id}"), &alice.token)
        .await;
    let purged = app
        .delete(&format!("/api/v1/trash/{folder_id}"), &alice.token)
        .await;

    assert_eq!(purged.status, StatusCode::NO_CONTENT);
    assert!(
        app.get("/api/v1/trash", &alice.token)
            .await
            .body
            .as_array()
            .unwrap()
            .is_empty()
    );
    for key in &keys {
        assert_eq!(app.state.storage.head(key).await.unwrap(), None, "{key}");
    }
}

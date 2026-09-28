mod common;

use axum::http::{Method, StatusCode};
use common::TestApp;
use serde_json::{Value, json};

fn names(body: &Value) -> Vec<&str> {
    body.as_array()
        .expect("a JSON array")
        .iter()
        .map(|node| node["name"].as_str().unwrap())
        .collect()
}

async fn create_folder(app: &TestApp, token: &str, parent_id: Option<&str>, name: &str) -> Value {
    let response = app
        .post(
            "/api/v1/folders",
            token,
            json!({ "parent_id": parent_id, "name": name }),
        )
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{:?}", response.body);
    response.body
}

#[tokio::test]
async fn folders_are_created_listed_and_navigated() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;

    let docs = create_folder(&app, &alice.token, None, "docs").await;
    create_folder(&app, &alice.token, None, "archive").await;
    let docs_id = docs["id"].as_str().unwrap();
    let reports = create_folder(&app, &alice.token, Some(docs_id), "reports").await;
    let reports_id = reports["id"].as_str().unwrap();

    assert_eq!(docs["type"], "folder");
    assert_eq!(reports["parent_id"], docs["id"]);

    let root = app.get("/api/v1/nodes", &alice.token).await;
    assert_eq!(names(&root.body), vec!["archive", "docs"]);

    let children = app
        .get(&format!("/api/v1/nodes/{docs_id}/children"), &alice.token)
        .await;
    assert_eq!(names(&children.body), vec!["reports"]);

    let path = app
        .get(&format!("/api/v1/nodes/{reports_id}/path"), &alice.token)
        .await;
    assert_eq!(names(&path.body), vec!["docs", "reports"]);

    let node = app
        .get(&format!("/api/v1/nodes/{reports_id}"), &alice.token)
        .await;
    assert_eq!(node.status, StatusCode::OK);
    assert_eq!(node.body["name"], "reports");
}

#[tokio::test]
async fn nodes_are_renamed_and_moved() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let source = create_folder(&app, &alice.token, None, "source").await;
    let target = create_folder(&app, &alice.token, None, "target").await;
    let item = create_folder(&app, &alice.token, source["id"].as_str(), "item").await;
    let item_id = item["id"].as_str().unwrap();

    let renamed = app
        .post(
            &format!("/api/v1/nodes/{item_id}/rename"),
            &alice.token,
            json!({ "name": "renamed" }),
        )
        .await;
    assert_eq!(renamed.status, StatusCode::OK);
    assert_eq!(renamed.body["name"], "renamed");

    let moved = app
        .post(
            &format!("/api/v1/nodes/{item_id}/move"),
            &alice.token,
            json!({ "parent_id": target["id"] }),
        )
        .await;
    assert_eq!(moved.status, StatusCode::OK);
    assert_eq!(moved.body["parent_id"], target["id"]);

    let to_root = app
        .post(
            &format!("/api/v1/nodes/{item_id}/move"),
            &alice.token,
            json!({ "parent_id": null }),
        )
        .await;
    assert_eq!(to_root.status, StatusCode::OK);
    assert_eq!(to_root.body["parent_id"], Value::Null);
}

#[tokio::test]
async fn domain_errors_are_reported_with_their_codes() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let parent = create_folder(&app, &alice.token, None, "parent").await;
    let parent_id = parent["id"].as_str().unwrap();
    let child = create_folder(&app, &alice.token, Some(parent_id), "child").await;

    let duplicate = app
        .post("/api/v1/folders", &alice.token, json!({ "name": "parent" }))
        .await;
    assert_eq!(duplicate.status, StatusCode::CONFLICT);
    assert_eq!(duplicate.body["error"]["code"], "name_conflict");

    let invalid = app
        .post("/api/v1/folders", &alice.token, json!({ "name": "a/b" }))
        .await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(invalid.body["error"]["code"], "invalid_name");

    let cycle = app
        .post(
            &format!("/api/v1/nodes/{parent_id}/move"),
            &alice.token,
            json!({ "parent_id": child["id"] }),
        )
        .await;
    assert_eq!(cycle.status, StatusCode::CONFLICT);
    assert_eq!(cycle.body["error"]["code"], "cycle_detected");

    let missing = app
        .get(
            &format!("/api/v1/nodes/{}", uuid::Uuid::new_v4()),
            &alice.token,
        )
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn other_users_nodes_are_invisible() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let secret = create_folder(&app, &alice.token, None, "secret").await;
    let secret_id = secret["id"].as_str().unwrap();
    let bobs = create_folder(&app, &bob.token, None, "bobs").await;
    let bobs_id = bobs["id"].as_str().unwrap();

    let attempts = [
        app.get(&format!("/api/v1/nodes/{secret_id}"), &bob.token)
            .await,
        app.get(&format!("/api/v1/nodes/{secret_id}/children"), &bob.token)
            .await,
        app.get(&format!("/api/v1/nodes/{secret_id}/path"), &bob.token)
            .await,
        app.post(
            &format!("/api/v1/nodes/{secret_id}/rename"),
            &bob.token,
            json!({ "name": "x" }),
        )
        .await,
        app.post(
            &format!("/api/v1/nodes/{secret_id}/move"),
            &bob.token,
            json!({ "parent_id": null }),
        )
        .await,
        app.post(
            "/api/v1/folders",
            &bob.token,
            json!({ "parent_id": secret_id, "name": "x" }),
        )
        .await,
        app.post(
            &format!("/api/v1/nodes/{bobs_id}/move"),
            &bob.token,
            json!({ "parent_id": secret_id }),
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

    let bobs_root = app.get("/api/v1/nodes", &bob.token).await;
    assert_eq!(names(&bobs_root.body), vec!["bobs"]);
    let unchanged = app
        .get(&format!("/api/v1/nodes/{secret_id}"), &alice.token)
        .await;
    assert_eq!(unchanged.body["name"], "secret");
}

#[tokio::test]
async fn node_routes_require_authentication() {
    let app = TestApp::start().await;

    let response = app.request(Method::GET, "/api/v1/nodes", None, None).await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

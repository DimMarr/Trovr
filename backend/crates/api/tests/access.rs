mod common;

use axum::http::StatusCode;
use common::{TestApp, TestUser};
use serde_json::{Value, json};
use trovr_metadata::Role;
use uuid::Uuid;

async fn folder(app: &TestApp, user: &TestUser, parent_id: Option<&str>, name: &str) -> Value {
    let response = app
        .post(
            "/api/v1/folders",
            &user.token,
            json!({ "parent_id": parent_id, "name": name }),
        )
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{:?}", response.body);
    response.body
}

async fn share(app: &TestApp, node: &Value, owner: &TestUser, with: &TestUser, role: Role) {
    let node_id = Uuid::parse_str(node["id"].as_str().unwrap()).unwrap();
    app.state
        .nodes
        .share_with_user(node_id, with.id, role, owner.id)
        .await
        .unwrap();
}

fn id(node: &Value) -> &str {
    node["id"].as_str().unwrap()
}

fn names(body: &Value) -> Vec<&str> {
    body.as_array()
        .unwrap()
        .iter()
        .map(|n| n["name"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn viewers_can_read_but_not_write() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let shared = folder(&app, &alice, None, "shared").await;
    let file = app
        .upload_file(&alice, Some(id(&shared)), "x.txt", b"data")
        .await;
    share(&app, &shared, &alice, &bob, Role::Viewer).await;
    let (f, x) = (id(&shared), id(&file));

    let node = app.get(&format!("/api/v1/nodes/{f}"), &bob.token).await;
    assert_eq!(node.status, StatusCode::OK);
    assert_eq!(node.body["role"], "viewer");
    let children = app
        .get(&format!("/api/v1/nodes/{f}/children"), &bob.token)
        .await;
    assert_eq!(names(&children.body), vec!["x.txt"]);
    for uri in [
        format!("/api/v1/nodes/{x}/path"),
        format!("/api/v1/nodes/{x}/versions"),
        format!("/api/v1/nodes/{x}/download"),
    ] {
        assert_eq!(
            app.get(&uri, &bob.token).await.status,
            StatusCode::OK,
            "{uri}"
        );
    }

    let bobs_key = app.upload(&bob, "text/plain", b"bob").await;
    let attempts = [
        app.post(
            &format!("/api/v1/nodes/{x}/rename"),
            &bob.token,
            json!({ "name": "y.txt" }),
        )
        .await,
        app.post(
            &format!("/api/v1/nodes/{x}/move"),
            &bob.token,
            json!({ "parent_id": null }),
        )
        .await,
        app.delete(&format!("/api/v1/nodes/{x}"), &bob.token).await,
        app.post(
            "/api/v1/folders",
            &bob.token,
            json!({ "parent_id": f, "name": "sub" }),
        )
        .await,
        app.post(
            "/api/v1/files",
            &bob.token,
            json!({ "storage_key": bobs_key, "parent_id": f, "name": "b.txt" }),
        )
        .await,
        app.post(
            &format!("/api/v1/nodes/{x}/versions"),
            &bob.token,
            json!({ "storage_key": bobs_key }),
        )
        .await,
    ];
    for response in attempts {
        assert_eq!(
            response.status,
            StatusCode::FORBIDDEN,
            "{:?}",
            response.body
        );
        assert_eq!(response.body["error"]["code"], "forbidden");
    }
}

#[tokio::test]
async fn editors_can_write_inside_shared_folders() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let shared = folder(&app, &alice, None, "shared").await;
    let file = app
        .upload_file(&alice, Some(id(&shared)), "x.txt", b"data")
        .await;
    share(&app, &shared, &alice, &bob, Role::Editor).await;
    let (f, x) = (id(&shared), id(&file));

    folder(&app, &bob, Some(f), "bobs").await;
    let uploaded = app.upload_file(&bob, Some(f), "from-bob.txt", b"hi").await;
    let alices_view = app
        .get(&format!("/api/v1/nodes/{}", id(&uploaded)), &alice.token)
        .await;
    assert_eq!(
        alices_view.body["role"], "owner",
        "folder owners own what sharees add"
    );

    let renamed = app
        .post(
            &format!("/api/v1/nodes/{x}/rename"),
            &bob.token,
            json!({ "name": "renamed.txt" }),
        )
        .await;
    assert_eq!(renamed.status, StatusCode::OK);
    let key = app.upload(&bob, "text/plain", b"v2").await;
    let version = app
        .post(
            &format!("/api/v1/nodes/{x}/versions"),
            &bob.token,
            json!({ "storage_key": key }),
        )
        .await;
    assert_eq!(version.status, StatusCode::CREATED);

    let trashed = app.delete(&format!("/api/v1/nodes/{x}"), &bob.token).await;
    assert_eq!(trashed.status, StatusCode::OK);
    let alices_trash = app.get("/api/v1/trash", &alice.token).await;
    assert_eq!(names(&alices_trash.body), vec!["renamed.txt"]);
    let restore_by_editor = app
        .post(&format!("/api/v1/nodes/{x}/restore"), &bob.token, json!({}))
        .await;
    assert_eq!(restore_by_editor.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn editors_cannot_carry_nodes_out() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let shared = folder(&app, &alice, None, "shared").await;
    let target = folder(&app, &alice, Some(id(&shared)), "target").await;
    let item = folder(&app, &alice, Some(id(&shared)), "item").await;
    let bobs_private = folder(&app, &bob, None, "private").await;
    share(&app, &shared, &alice, &bob, Role::Editor).await;
    let move_uri = format!("/api/v1/nodes/{}/move", id(&item));

    let out = app
        .post(
            &move_uri,
            &bob.token,
            json!({ "parent_id": id(&bobs_private) }),
        )
        .await;
    assert_eq!(out.status, StatusCode::FORBIDDEN);
    let to_root = app
        .post(&move_uri, &bob.token, json!({ "parent_id": null }))
        .await;
    assert_eq!(to_root.status, StatusCode::FORBIDDEN);
    let inside = app
        .post(&move_uri, &bob.token, json!({ "parent_id": id(&target) }))
        .await;
    assert_eq!(inside.status, StatusCode::OK);
}

#[tokio::test]
async fn sharees_see_a_truncated_path() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let a = folder(&app, &alice, None, "a").await;
    let b = folder(&app, &alice, Some(id(&a)), "b").await;
    let c = folder(&app, &alice, Some(id(&b)), "c").await;
    share(&app, &b, &alice, &bob, Role::Viewer).await;
    let uri = format!("/api/v1/nodes/{}/path", id(&c));

    assert_eq!(names(&app.get(&uri, &bob.token).await.body), vec!["b", "c"]);
    assert_eq!(
        names(&app.get(&uri, &alice.token).await.body),
        vec!["a", "b", "c"]
    );
    let parent = app
        .get(&format!("/api/v1/nodes/{}", id(&a)), &bob.token)
        .await;
    assert_eq!(parent.status, StatusCode::NOT_FOUND);
}

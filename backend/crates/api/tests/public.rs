mod common;

use std::time::Duration;

use axum::http::{Method, StatusCode};
use common::{TestApp, TestResponse, TestUser};
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

async fn create_link(app: &TestApp, node: &Value, owner: &TestUser, body: Value) -> Value {
    let response = app
        .post(
            &format!("/api/v1/nodes/{}/links", id(node)),
            &owner.token,
            body,
        )
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{:?}", response.body);
    response.body
}

async fn anonymous(app: &TestApp, uri: &str) -> TestResponse {
    app.request(Method::GET, uri, None, None).await
}

fn token(link: &Value) -> &str {
    link["token"].as_str().unwrap()
}

#[tokio::test]
async fn public_links_browse_and_download_without_an_account() {
    let app = TestApp::start_with_storage().await;
    let alice = app.create_user("alice@example.com").await;
    let shared = folder(&app, &alice, None, "shared").await;
    let sub = folder(&app, &alice, Some(id(&shared)), "sub").await;
    let file = app
        .upload_file(&alice, Some(id(&sub)), "x.txt", b"public")
        .await;

    let link = create_link(&app, &shared, &alice, json!({})).await;
    assert!(token(&link).len() >= 60);
    assert_eq!(link["role"], "viewer");
    assert_eq!(link["expires_at"], Value::Null);
    let t = token(&link);

    let root = anonymous(&app, &format!("/api/v1/public/{t}")).await;
    assert_eq!(root.status, StatusCode::OK);
    assert_eq!(root.body["name"], "shared");
    let top = anonymous(
        &app,
        &format!("/api/v1/public/{t}/nodes/{}/children", id(&shared)),
    )
    .await;
    assert_eq!(names(&top.body), vec!["sub"]);
    let inner = anonymous(
        &app,
        &format!("/api/v1/public/{t}/nodes/{}/children", id(&sub)),
    )
    .await;
    assert_eq!(names(&inner.body), vec!["x.txt"]);

    let download = anonymous(
        &app,
        &format!("/api/v1/public/{t}/nodes/{}/download", id(&file)),
    )
    .await;
    assert_eq!(download.status, StatusCode::OK, "{:?}", download.body);
    let response = app.fetch(&download.body).await;
    assert_eq!(response.bytes().await.unwrap().as_ref(), b"public");

    let listing = app
        .get(
            &format!("/api/v1/nodes/{}/shares", id(&shared)),
            &alice.token,
        )
        .await;
    assert_eq!(listing.body["links"][0]["id"], link["id"]);
}

#[tokio::test]
async fn public_links_are_confined_to_their_subtree() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let parent = folder(&app, &alice, None, "parent").await;
    let shared = folder(&app, &alice, Some(id(&parent)), "shared").await;
    let sibling = folder(&app, &alice, Some(id(&parent)), "sibling").await;
    let trashed = folder(&app, &alice, Some(id(&shared)), "trashed").await;
    let under_trashed = folder(&app, &alice, Some(id(&trashed)), "deeper").await;
    let link = create_link(&app, &shared, &alice, json!({})).await;
    let t = token(&link);
    let trash = app
        .delete(&format!("/api/v1/nodes/{}", id(&trashed)), &alice.token)
        .await;
    assert_eq!(trash.status, StatusCode::OK);

    let visible = anonymous(
        &app,
        &format!("/api/v1/public/{t}/nodes/{}/children", id(&shared)),
    )
    .await;
    assert_eq!(visible.status, StatusCode::OK);
    assert!(
        names(&visible.body).is_empty(),
        "trashed children are hidden"
    );

    let outside = [
        id(&parent).to_string(),
        id(&sibling).to_string(),
        id(&trashed).to_string(),
        id(&under_trashed).to_string(),
        Uuid::new_v4().to_string(),
    ];
    for node_id in outside {
        for action in ["children", "download"] {
            let response = anonymous(
                &app,
                &format!("/api/v1/public/{t}/nodes/{node_id}/{action}"),
            )
            .await;
            assert_eq!(response.status, StatusCode::NOT_FOUND, "{node_id} {action}");
        }
    }
}

#[tokio::test]
async fn revoked_and_expired_links_are_dead() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let shared = folder(&app, &alice, None, "shared").await;

    let link = create_link(&app, &shared, &alice, json!({})).await;
    let public_uri = format!("/api/v1/public/{}", token(&link));
    assert_eq!(anonymous(&app, &public_uri).await.status, StatusCode::OK);
    let delete_uri = format!(
        "/api/v1/nodes/{}/links/{}",
        id(&shared),
        link["id"].as_str().unwrap()
    );
    assert_eq!(
        app.delete(&delete_uri, &alice.token).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        anonymous(&app, &public_uri).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.delete(&delete_uri, &alice.token).await.status,
        StatusCode::NOT_FOUND
    );

    let expiring = create_link(&app, &shared, &alice, json!({ "expires_in_seconds": 1 })).await;
    assert!(expiring["expires_at"].is_string());
    let expiring_uri = format!("/api/v1/public/{}", token(&expiring));
    assert_eq!(anonymous(&app, &expiring_uri).await.status, StatusCode::OK);
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(
        anonymous(&app, &expiring_uri).await.status,
        StatusCode::NOT_FOUND
    );

    let unknown = anonymous(&app, "/api/v1/public/not-a-token").await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn only_owners_manage_links() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let carol = app.create_user("carol@example.com").await;
    let shared = folder(&app, &alice, None, "shared").await;
    app.state
        .nodes
        .share_with_user(
            Uuid::parse_str(id(&shared)).unwrap(),
            bob.id,
            Role::Editor,
            alice.id,
        )
        .await
        .unwrap();
    let link = create_link(&app, &shared, &alice, json!({})).await;
    let links_uri = format!("/api/v1/nodes/{}/links", id(&shared));
    let delete_uri = format!("{links_uri}/{}", link["id"].as_str().unwrap());

    assert_eq!(
        app.post(&links_uri, &bob.token, json!({})).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.delete(&delete_uri, &bob.token).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.post(&links_uri, &carol.token, json!({})).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.delete(&delete_uri, &carol.token).await.status,
        StatusCode::NOT_FOUND
    );

    for seconds in [0, -5] {
        let response = app
            .post(
                &links_uri,
                &alice.token,
                json!({ "expires_in_seconds": seconds }),
            )
            .await;
        assert_eq!(
            response.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{seconds}"
        );
        assert_eq!(response.body["error"]["code"], "invalid_request");
    }
}

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

async fn share(app: &TestApp, node: &Value, by: &TestUser, email: &str, role: &str) -> Value {
    let response = app
        .post(
            &format!("/api/v1/nodes/{}/shares", id(node)),
            &by.token,
            json!({ "email": email, "role": role }),
        )
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{:?}", response.body);
    response.body
}

#[tokio::test]
async fn sharing_by_email_grants_access() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let docs = folder(&app, &alice, None, "docs").await;
    folder(&app, &alice, None, "private").await;

    let created = share(&app, &docs, &alice, "  Bob@Example.com ", "viewer").await;
    assert_eq!(created["user"]["id"], bob.id.to_string());
    assert_eq!(created["user"]["email"], "bob@example.com");
    assert_eq!(created["role"], "viewer");

    let shared = app.get("/api/v1/shared", &bob.token).await;
    assert_eq!(shared.status, StatusCode::OK);
    assert_eq!(names(&shared.body), vec!["docs"]);
    let opened = app
        .get(&format!("/api/v1/nodes/{}", id(&docs)), &bob.token)
        .await;
    assert_eq!(opened.status, StatusCode::OK);
    assert_eq!(opened.body["role"], "viewer");
    assert!(names(&app.get("/api/v1/shared", &alice.token).await.body).is_empty());

    let listing = app
        .get(&format!("/api/v1/nodes/{}/shares", id(&docs)), &alice.token)
        .await;
    assert_eq!(listing.status, StatusCode::OK);
    let users = listing.body["users"].as_array().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["user"]["email"], "bob@example.com");
    assert_eq!(users[0]["role"], "viewer");
    assert_eq!(listing.body["links"], json!([]));
}

#[tokio::test]
async fn share_requests_are_validated() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let carol = app.create_user("carol@example.com").await;
    let docs = folder(&app, &alice, None, "docs").await;
    let uri = format!("/api/v1/nodes/{}/shares", id(&docs));

    let unknown = app
        .post(
            &uri,
            &alice.token,
            json!({ "email": "nobody@example.com", "role": "viewer" }),
        )
        .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(unknown.body["error"]["code"], "user_not_found");

    for role in ["owner", "garbage"] {
        let response = app
            .post(
                &uri,
                &alice.token,
                json!({ "email": "bob@example.com", "role": role }),
            )
            .await;
        assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY, "{role}");
        assert_eq!(response.body["error"]["code"], "invalid_request");
    }

    let to_self = app
        .post(
            &uri,
            &alice.token,
            json!({ "email": "alice@example.com", "role": "viewer" }),
        )
        .await;
    assert_eq!(to_self.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Bob, as an editor, adds a folder inside docs: it is his, so Alice
    // cannot grant him a lesser role on it.
    app.state
        .nodes
        .share_with_user(
            Uuid::parse_str(id(&docs)).unwrap(),
            bob.id,
            Role::Editor,
            alice.id,
        )
        .await
        .unwrap();
    let bobs = folder(&app, &bob, Some(id(&docs)), "bobs").await;
    let to_owner = app
        .post(
            &format!("/api/v1/nodes/{}/shares", id(&bobs)),
            &alice.token,
            json!({ "email": "bob@example.com", "role": "viewer" }),
        )
        .await;
    assert_eq!(to_owner.status, StatusCode::UNPROCESSABLE_ENTITY);

    let by_editor = app
        .post(
            &uri,
            &bob.token,
            json!({ "email": "carol@example.com", "role": "viewer" }),
        )
        .await;
    assert_eq!(by_editor.status, StatusCode::FORBIDDEN);
    assert_eq!(
        app.get(&uri, &bob.token).await.status,
        StatusCode::FORBIDDEN
    );
    let revoke_by_editor = app.delete(&format!("{uri}/{}", bob.id), &bob.token).await;
    assert_eq!(revoke_by_editor.status, StatusCode::FORBIDDEN);

    let by_stranger = app
        .post(
            &uri,
            &carol.token,
            json!({ "email": "bob@example.com", "role": "viewer" }),
        )
        .await;
    assert_eq!(by_stranger.status, StatusCode::NOT_FOUND);
    assert_eq!(
        app.get(&uri, &carol.token).await.status,
        StatusCode::NOT_FOUND
    );

    sqlx::query(
        "INSERT INTO users (issuer, subject, email, display_name) \
         VALUES ('https://idp.example.com', 'carol', 'carol@example.com', 'Carol')",
    )
    .execute(&app.state.pool)
    .await
    .unwrap();
    let ambiguous = app
        .post(
            &uri,
            &alice.token,
            json!({ "email": "carol@example.com", "role": "viewer" }),
        )
        .await;
    assert_eq!(ambiguous.status, StatusCode::CONFLICT);
    assert_eq!(ambiguous.body["error"]["code"], "ambiguous_user");
}

#[tokio::test]
async fn revoking_a_share_cuts_access() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let docs = folder(&app, &alice, None, "docs").await;
    let sub = folder(&app, &alice, Some(id(&docs)), "sub").await;
    share(&app, &docs, &alice, "bob@example.com", "viewer").await;
    let sub_uri = format!("/api/v1/nodes/{}", id(&sub));
    assert_eq!(app.get(&sub_uri, &bob.token).await.status, StatusCode::OK);

    let revoke_uri = format!("/api/v1/nodes/{}/shares/{}", id(&docs), bob.id);
    assert_eq!(
        app.delete(&revoke_uri, &alice.token).await.status,
        StatusCode::NO_CONTENT
    );

    let docs_uri = format!("/api/v1/nodes/{}", id(&docs));
    assert_eq!(
        app.get(&docs_uri, &bob.token).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.get(&sub_uri, &bob.token).await.status,
        StatusCode::NOT_FOUND
    );
    assert!(names(&app.get("/api/v1/shared", &bob.token).await.body).is_empty());
    assert_eq!(
        app.delete(&revoke_uri, &alice.token).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn sharing_again_changes_the_role() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;
    let bob = app.create_user("bob@example.com").await;
    let docs = folder(&app, &alice, None, "docs").await;

    share(&app, &docs, &alice, "bob@example.com", "viewer").await;
    let updated = share(&app, &docs, &alice, "bob@example.com", "editor").await;
    assert_eq!(updated["role"], "editor");

    let opened = app
        .get(&format!("/api/v1/nodes/{}", id(&docs)), &bob.token)
        .await;
    assert_eq!(opened.body["role"], "editor");
    let listing = app
        .get(&format!("/api/v1/nodes/{}/shares", id(&docs)), &alice.token)
        .await;
    let users = listing.body["users"].as_array().unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["role"], "editor");
}

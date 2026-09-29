mod common;

use axum::http::{Method, StatusCode};
use common::{TestApp, TestOptions, TestResponse};
use serde_json::json;

async fn register(app: &TestApp, email: &str, password: &str, display_name: &str) -> TestResponse {
    app.request(
        Method::POST,
        "/api/v1/auth/register",
        None,
        Some(json!({ "email": email, "password": password, "display_name": display_name })),
    )
    .await
}

async fn login(app: &TestApp, email: &str, password: &str) -> TestResponse {
    app.request(
        Method::POST,
        "/api/v1/auth/login",
        None,
        Some(json!({ "email": email, "password": password })),
    )
    .await
}

#[tokio::test]
async fn register_then_login_then_me() {
    let app = TestApp::start().await;

    let registered = register(
        &app,
        " Alice@Example.com ",
        "correct horse battery",
        "Alice",
    )
    .await;
    assert_eq!(
        registered.status,
        StatusCode::CREATED,
        "{:?}",
        registered.body
    );
    assert_eq!(registered.body["email"], "alice@example.com");
    assert_eq!(registered.body["display_name"], "Alice");

    let logged_in = login(&app, "alice@example.com", "correct horse battery").await;
    assert_eq!(logged_in.status, StatusCode::OK);
    assert_eq!(logged_in.body["token_type"], "Bearer");

    let token = logged_in.body["access_token"].as_str().unwrap();
    let me = app.get("/api/v1/me", token).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["id"], registered.body["id"]);
}

#[tokio::test]
async fn registering_a_taken_email_is_a_conflict() {
    let app = TestApp::start().await;
    register(&app, "bob@example.com", "correct horse battery", "Bob").await;

    let again = register(&app, "BOB@example.com", "another password", "Bob 2").await;

    assert_eq!(again.status, StatusCode::CONFLICT);
    assert_eq!(again.body["error"]["code"], "email_taken");
}

#[tokio::test]
async fn registration_validates_its_input() {
    let app = TestApp::start().await;

    for (email, password, display_name) in [
        ("not-an-email", "correct horse battery", "Carol"),
        ("carol@", "correct horse battery", "Carol"),
        ("carol@example.com", "short", "Carol"),
        ("carol@example.com", "correct horse battery", "   "),
    ] {
        let response = register(&app, email, password, display_name).await;
        assert_eq!(
            response.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{email:?} {password:?} {display_name:?}"
        );
        assert_eq!(response.body["error"]["code"], "invalid_request");
    }
}

#[tokio::test]
async fn login_with_wrong_credentials_is_unauthorized() {
    let app = TestApp::start().await;
    register(&app, "dave@example.com", "correct horse battery", "Dave").await;

    for (email, password) in [
        ("dave@example.com", "wrong password"),
        ("nobody@example.com", "correct horse battery"),
    ] {
        let response = login(&app, email, password).await;
        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
        assert_eq!(response.body["error"]["code"], "invalid_credentials");
    }
}

#[tokio::test]
async fn registration_can_be_disabled() {
    let mut options = TestOptions::default();
    options.settings.allow_registration = false;
    let app = TestApp::start_with(options).await;
    app.create_user("erin@example.com").await;

    let response = register(&app, "frank@example.com", "correct horse battery", "Frank").await;
    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert_eq!(response.body["error"]["code"], "registration_disabled");

    let existing = login(&app, "erin@example.com", common::PASSWORD).await;
    assert_eq!(
        existing.status,
        StatusCode::OK,
        "login still works for existing accounts"
    );
}

#[tokio::test]
async fn internal_endpoints_do_not_exist_in_oidc_mode() {
    let app = TestApp::start_with(TestOptions {
        internal_auth: false,
        oidc_issuer: Some("http://127.0.0.1:9".to_string()),
        ..TestOptions::default()
    })
    .await;

    let registered = register(&app, "gina@example.com", "correct horse battery", "Gina").await;
    let logged_in = login(&app, "gina@example.com", "correct horse battery").await;

    assert_eq!(registered.status, StatusCode::NOT_FOUND);
    assert_eq!(logged_in.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn auth_config_describes_enabled_sign_in_methods() {
    let internal = TestApp::start().await;
    let config = internal
        .request(Method::GET, "/api/v1/auth/config", None, None)
        .await;
    assert_eq!(config.status, StatusCode::OK);
    assert_eq!(
        config.body,
        json!({
            "internal": { "enabled": true, "registration": true },
            "oidc": null,
            "max_upload_bytes": common::MAX_UPLOAD_BYTES,
        })
    );

    let closed = TestApp::start_with(TestOptions {
        settings: trovr_api::ApiSettings {
            allow_registration: false,
            ..TestOptions::default().settings
        },
        ..TestOptions::default()
    })
    .await;
    let config = closed
        .request(Method::GET, "/api/v1/auth/config", None, None)
        .await;
    assert_eq!(config.body["internal"]["registration"], false);

    let oidc = TestApp::start_with(TestOptions {
        internal_auth: false,
        oidc_issuer: Some("https://idp.example.com/realms/trovr/".to_string()),
        ..TestOptions::default()
    })
    .await;
    let config = oidc
        .request(Method::GET, "/api/v1/auth/config", None, None)
        .await;
    assert_eq!(
        config.body["internal"],
        json!({ "enabled": false, "registration": false })
    );
    assert_eq!(
        config.body["oidc"],
        json!({ "issuer": "https://idp.example.com/realms/trovr", "client_id": "trovr" })
    );
}

mod common;

use axum::http::header::WWW_AUTHENTICATE;
use axum::http::{Method, StatusCode};
use common::{JWT_PRIVATE_KEY_PEM, JWT_PUBLIC_KEY_PEM, TestApp, TestOptions};
use trovr_auth::InternalValidator;
use uuid::Uuid;

#[tokio::test]
async fn health_endpoints_report_ok() {
    let app = TestApp::start().await;

    let live = app.request(Method::GET, "/healthz", None, None).await;
    let ready = app.request(Method::GET, "/readyz", None, None).await;

    assert_eq!(live.status, StatusCode::OK);
    assert_eq!(ready.status, StatusCode::OK);
}

#[tokio::test]
async fn protected_routes_require_a_bearer_token() {
    let app = TestApp::start().await;

    let response = app.request(Method::GET, "/api/v1/me", None, None).await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    assert_eq!(response.headers[WWW_AUTHENTICATE], "Bearer");
    assert_eq!(response.body["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn malformed_and_forged_tokens_are_refused() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;

    let mut forged = alice.token.clone();
    let signature_char = forged.len() - 10;
    let replacement = if &forged[signature_char..=signature_char] == "A" {
        "B"
    } else {
        "A"
    };
    forged.replace_range(signature_char..=signature_char, replacement);

    for authorization in [
        "Bearer not-a-jwt".to_string(),
        format!("Basic {}", alice.token),
        format!("Bearer {forged}"),
        "Bearer ".to_string(),
    ] {
        let response = app
            .get_with_authorization("/api/v1/me", &authorization)
            .await;
        assert_eq!(response.status, StatusCode::UNAUTHORIZED, "{authorization}");
    }
}

#[tokio::test]
async fn me_returns_the_authenticated_user() {
    let app = TestApp::start().await;
    let alice = app.create_user("alice@example.com").await;

    let response = app.get("/api/v1/me", &alice.token).await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["id"], alice.id.to_string());
    assert_eq!(response.body["email"], "alice@example.com");
    assert_eq!(response.body["issuer"], "internal");
}

#[tokio::test]
async fn internal_tokens_are_refused_when_only_oidc_is_enabled() {
    let app = TestApp::start_with(TestOptions {
        internal_auth: false,
        // Never contacted: a token without a `kid` is refused before any
        // discovery request.
        oidc_issuer: Some("http://127.0.0.1:9".to_string()),
        ..TestOptions::default()
    })
    .await;
    let token = InternalValidator::new(JWT_PRIVATE_KEY_PEM, JWT_PUBLIC_KEY_PEM)
        .unwrap()
        .issue_token(Uuid::new_v4(), "a@example.com", "A")
        .unwrap();

    let response = app.get("/api/v1/me", &token).await;

    assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

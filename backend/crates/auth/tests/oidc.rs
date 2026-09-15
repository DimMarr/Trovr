use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use trovr_auth::{OidcValidator, TokenValidator};

const TEST_OIDC_PRIVATE_KEY_PEM: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEAnyfoLdjOk+NeGtCKrspEkoM2ztxEwT3yv2Nakbxf3MMUb5+L
VqVTBQztf8MRbxBd6adbINnuBWFblIg67zaJ7ZvTwbMUUsFXPdn+9RzDs6TReF51
LAXtdPpWUDSb2d6wzVkV2fqlbEDsgbTIxMUV7GVAPgPuYCpuoLq+YyYObdWMFrHJ
XFSQgSNOpvmOr3noiZxSoNKl427Mrol4fkwnC6xr5Yigy/DRq6ro+q6RKqds6GHu
1Bg+lz5C6MmGdNlU4dZdYAdQzKpI/ONXvvZIUNRd17Cb347QYc5z+eWBCgM2ZFnX
Tj3rStkWbXyGJFcgcYtfyBULw4Ic0z5IQB0kOwIDAQABAoIBAADbq/1+ZrLn5ATJ
U1exC44/EY0O4Sr/SbfSBgNI8Ha1TI0TEpejbxs1rQAh+fz23+0PQLNQ8U3R1VM/
isili8ZoSOgdagESCoWV3CjP6bN+X/EDdB4Svt0mNMdwZDJSgfJj+VYb2hRRWVaD
ESkFobeHM6VYtNt01MyvbOPpoGGlDaSS9haUEz+GBlFidQYNk5T3ifp9b30uPTfz
yUG0pTOEbWZz7arc/ac5oonsP01L5j8n5jtoAE8KnZbC70XJSsImf00/LiDCqTMn
EFRZjm2xCQWcKpMm7jtanHtLWJDe4XGvOfJZNliBaQxURp77+CbUeugeI9ow5fXl
djPsufECgYEA2p4p00J6fYHZGYfpx7br/CRHTpA2Qn0Tg+PwfrGZzytgvyrOvaKw
jJJIs/kfKQSjRAgvyVVkP+5jB4UJUyZhBz3vM4z8dFahLsP5a5zdpa6JybDAKPQ7
Vd0ophDR9ebIYtu2gb8Tg027v1Af/LIeP4oZlyqQXvGv/MxcNuRdn/ECgYEAul7X
IPqi1u/H739oIPSHfYHIL/kcVciWUvb/jyym5i4WN7piJrDswTQUAsGKRN+mUi5L
4hqTlRel2JueAuLmAFQ1EPlfseUomVS+uw7mJyjbuDlNSw7hGU8LFJ4/RJ+HfhNO
PyRWwAkTnf/AhcOVRGc/lL4F0dwOV6w9Ro4ScusCgYEArGkR6TdDbNnLwnPKriOX
xnlB9zaKZDNKAbjxAKnF+HloSjtTYoD8pU/0oGL27R1oOQ4PycNbVYQGe3ay3O2m
ldSFYe1tZ76uTThm6zSCJNkad4K6eVHrvZK2LQmU3E5OeJB4RajQlbvnNkDViS9b
5ZZfCjWaOBZ5SXBNxUiigbECgYB9fGQ/mWLRdAvcD54uKlecQylmW8YfYHsPC65R
WdBaxgdBqKZzxMb047rhjC0saKZVSUTgzeI3DgAmE8nVya7x48EDV9V+M30dmLj/
vG1tSo5+wV5wvmkEIHume1LAQuX5FsilrTMYBESIKu8XYfR8ZUSjQzsp7ZBeqeNs
QTbc1wKBgE/OrbaftELLXHTllg54zzTJf3obqqZqudR3wdiu7R6Pqc/qX95Rs/Ju
mk+ATqQ3NqvHZSMvpWjkQ0XgdVHoxiEYMzWAyRh6D+yvThmkQV+Y34WlbvvU1ap3
A4HiKirkx2vtxPLkMb2cYugSSjmeF75mK/0Vf+RItd5dxT5Zko3h
-----END RSA PRIVATE KEY-----
";

const TEST_JWKS_JSON: &str = r#"{
  "keys": [
    {
      "use": "sig",
      "alg": "RS256",
      "kid": "test-key-1",
      "kty": "RSA",
      "n": "nyfoLdjOk-NeGtCKrspEkoM2ztxEwT3yv2Nakbxf3MMUb5-LVqVTBQztf8MRbxBd6adbINnuBWFblIg67zaJ7ZvTwbMUUsFXPdn-9RzDs6TReF51LAXtdPpWUDSb2d6wzVkV2fqlbEDsgbTIxMUV7GVAPgPuYCpuoLq-YyYObdWMFrHJXFSQgSNOpvmOr3noiZxSoNKl427Mrol4fkwnC6xr5Yigy_DRq6ro-q6RKqds6GHu1Bg-lz5C6MmGdNlU4dZdYAdQzKpI_ONXvvZIUNRd17Cb347QYc5z-eWBCgM2ZFnXTj3rStkWbXyGJFcgcYtfyBULw4Ic0z5IQB0kOw",
      "e": "AQAB"
    }
  ]
}"#;

#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: String,
    iss: String,
    aud: String,
    email: String,
    name: String,
    exp: i64,
    iat: i64,
}

async fn start_migrated_postgres() -> (PgPool, ContainerAsync<Postgres>) {
    let container = Postgres::default()
        .with_tag("16-alpine")
        .start()
        .await
        .expect("postgres container should start");
    let host_port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port should be published");
    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{host_port}/postgres");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("should connect to postgres");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("migrations should apply cleanly");

    (pool, container)
}

async fn start_mock_idp() -> MockServer {
    let mock_server = MockServer::start().await;
    let issuer = mock_server.uri();

    let discovery_body = json!({
        "issuer": issuer,
        "jwks_uri": format!("{issuer}/jwks"),
    });
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&discovery_body))
        .mount(&mock_server)
        .await;

    let jwks_value: serde_json::Value =
        serde_json::from_str(TEST_JWKS_JSON).expect("fixture JWKS JSON should parse");
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jwks_value))
        .mount(&mock_server)
        .await;

    mock_server
}

fn sign_test_token(issuer: &str, client_id: &str, sub: &str, email: &str, name: &str) -> String {
    let encoding_key = EncodingKey::from_rsa_pem(TEST_OIDC_PRIVATE_KEY_PEM.as_bytes())
        .expect("test private key should parse");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_secs() as i64;

    let claims = TestClaims {
        sub: sub.to_string(),
        iss: issuer.to_string(),
        aud: client_id.to_string(),
        email: email.to_string(),
        name: name.to_string(),
        iat: now,
        exp: now + 3600,
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("test-key-1".to_string());

    encode(&header, &claims, &encoding_key).expect("token should encode")
}

#[tokio::test]
async fn validate_verifies_token_and_creates_a_new_user() {
    let (pool, _container) = start_migrated_postgres().await;
    let mock_server = start_mock_idp().await;
    let issuer = mock_server.uri();

    let validator = OidcValidator::new(pool.clone(), issuer.clone(), "trovr-client".to_string());
    let token = sign_test_token(&issuer, "trovr-client", "user-42", "bob@example.com", "Bob");

    let user = validator.validate(&token).await.expect("token should validate");

    assert_eq!(user.subject, "user-42");
    assert_eq!(user.issuer, issuer);
    assert_eq!(user.email, "bob@example.com");
    assert_eq!(user.display_name, "Bob");

    let row_count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE issuer = $1 AND subject = $2")
        .bind(&issuer)
        .bind("user-42")
        .fetch_one(&pool)
        .await
        .expect("count query should succeed");
    assert_eq!(row_count, 1);
}

#[tokio::test]
async fn validate_upserts_the_same_user_on_a_second_login() {
    let (pool, _container) = start_migrated_postgres().await;
    let mock_server = start_mock_idp().await;
    let issuer = mock_server.uri();

    let validator = OidcValidator::new(pool.clone(), issuer.clone(), "trovr-client".to_string());

    let first_token = sign_test_token(&issuer, "trovr-client", "user-99", "old@example.com", "Old Name");
    let first_user = validator.validate(&first_token).await.expect("first token should validate");

    let second_token = sign_test_token(&issuer, "trovr-client", "user-99", "new@example.com", "New Name");
    let second_user = validator.validate(&second_token).await.expect("second token should validate");

    assert_eq!(first_user.user_id, second_user.user_id);
    assert_eq!(second_user.email, "new@example.com");
    assert_eq!(second_user.display_name, "New Name");

    let row_count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE issuer = $1 AND subject = $2")
        .bind(&issuer)
        .bind("user-99")
        .fetch_one(&pool)
        .await
        .expect("count query should succeed");
    assert_eq!(row_count, 1);
}

#[tokio::test]
async fn validate_rejects_a_token_with_the_wrong_audience() {
    let (pool, _container) = start_migrated_postgres().await;
    let mock_server = start_mock_idp().await;
    let issuer = mock_server.uri();

    let validator = OidcValidator::new(pool.clone(), issuer.clone(), "trovr-client".to_string());
    let token = sign_test_token(&issuer, "some-other-client", "user-1", "eve@example.com", "Eve");

    let result = validator.validate(&token).await;

    assert!(result.is_err());
}

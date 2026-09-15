use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use trovr_auth::{AuthError, InternalValidator, TokenValidator};

const TEST_PRIVATE_KEY_PEM: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEAw9lUtcAXHym8WbGzNN96QjBZtek8T3hYse3BAW+2nzMRsw1b
S0o6OI6OKH/36m/mUSwJ+5A3xzn17FOKiJizJuv4qLVss3Ig/MMXnFycBsvp0ceL
sP17loF3wHfUEjQAbqZSMK02jXqoLOPIfy9HcpeAZTAbabIJLLFLWaZcLCZnuial
731FbGTSLfgS2xcGDDpvuhnJ478jJ60XpRxFnMCqkr7srdQlJ0Dbur9IZ3mb/yaW
YI13vw2Fx6Fd7cNyOi7v8NToAO+A9auPvT1rAFa93+Yhtmd+6rUlqT6vlenQ2ti8
LHK+JyIMitqrblbzQepC4flnO+OO3YGzRxTBmQIDAQABAoIBAA8OL/Lg12Yv4SjL
/ki8PTvFV8AiM96wE7Fp44JmwhUu6ddn2XLKO/uJCeiXHcnJ3Fy1E/dguMj57avD
KE0j3/HIBg+CEt8gkOSdYr1EaUIkfs7/lf9/HPcO8NO84nKbyMS4yGyxhBYQLcl1
oT4VtlpaxyYAMrtHUmk7H92+aiuCjaGBuO5iCUr1ckWzaAamws+rZlAi3R36CnS0
+JtnTUuCyfjsJ40ZOcRAVNOQyaQTIlyfl8YpGCOB5PriaybxV2N3Th/+4EN6Npsg
b+QkzzhWzc5zwZ1x+Kry1gLRlTNqLsfsT8dLt235zdlpD67fRBY54/JYlBhlrZGw
dgXgS1kCgYEA+6YL06vtVe8KkUHT6gViqSJaQb76VsFnO/Vr5RT3fMpBkYxoEhDE
ZQ40ZacykKtvfHSzaRSnBdGStaOgdDpgrk6SJ6SpScFXJSJTlL0Ztr3yv1+TYmKx
TL5n4xWD6HC7zdHi1VGnmrULQ4U4iTy8XBaOoUcApKF2/GMv+fdnkh8CgYEAxzxH
0fQkVAzqMojeDOJeNcPbM2QLXYvgKe17XQY9V8Vh2vHn2j62LCeTkjuCSlbhczFT
JN+5aKYirVgrSi2EYyE8j9P+44pebTzHuBmcxmRvRHV4pqwMEMrduc3HlP37mDgP
bOvkgi2hhA6Hwaahc9Qpnyd+/ONNedB/eI6rZUcCgYBp1hUFSjrAOI/eNaxVsTwk
XDFPk22gDSlI21gseZv43OuktkOSzYB76/R3iFFI7QEve7l1CV2RoemtAQtbtq2w
wZTQnX0havImyQAT/1AQPmUYva6z40QkPbRdmk/m83rY/lwDUZtHAruhAyea+HNT
25zTEZSgqNPtJB4qaDYkCwKBgQDBZtDJDFgfhxHo0FfM5glR47frYRhvTJLj4HY6
TQ5LH33oTZ8lim7I2fo0n7PQehoL+judtdeDsJJE9yu+rASxPdhOPhpVw0H6hF/T
ZHl9VI12RRpDoQttWaB29zzgctRCZVkEANEnVShOytQZtalQiQmGR47L6dKRh0XW
P5g9jQKBgDciIXlpFEBn9E/qzIWJE/IQuO/qtTX/GZ2g7aoEpsU0mjSXzFSJTN7R
/99UjDOdv/Q5Gglk3aksBmzWHjtGW3rMFKVAm5dbOMoWExHkThjoNGQ49HOuuJxK
25CeDRyPEhFYWbMwlwNk7zodhhinerIipZz/NN1zBDC55HoRlfCK
-----END RSA PRIVATE KEY-----
";

const TEST_PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAw9lUtcAXHym8WbGzNN96
QjBZtek8T3hYse3BAW+2nzMRsw1bS0o6OI6OKH/36m/mUSwJ+5A3xzn17FOKiJiz
Juv4qLVss3Ig/MMXnFycBsvp0ceLsP17loF3wHfUEjQAbqZSMK02jXqoLOPIfy9H
cpeAZTAbabIJLLFLWaZcLCZnuial731FbGTSLfgS2xcGDDpvuhnJ478jJ60XpRxF
nMCqkr7srdQlJ0Dbur9IZ3mb/yaWYI13vw2Fx6Fd7cNyOi7v8NToAO+A9auPvT1r
AFa93+Yhtmd+6rUlqT6vlenQ2ti8LHK+JyIMitqrblbzQepC4flnO+OO3YGzRxTB
mQIDAQAB
-----END PUBLIC KEY-----
";

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

#[tokio::test]
async fn create_user_then_login_succeeds_and_token_validates() {
    let (pool, _container) = start_migrated_postgres().await;
    let validator = InternalValidator::new(TEST_PRIVATE_KEY_PEM, TEST_PUBLIC_KEY_PEM)
        .expect("validator should build from valid PEM keys");

    validator
        .create_internal_user(&pool, "alice@example.com", "Alice", "correct horse battery staple")
        .await
        .expect("user creation should succeed");

    let token = validator
        .login(&pool, "alice@example.com", "correct horse battery staple")
        .await
        .expect("login should succeed with the right password");

    let user = validator.validate(&token).await.expect("issued token should validate");

    assert_eq!(user.email, "alice@example.com");
    assert_eq!(user.display_name, "Alice");
    assert_eq!(user.issuer, "internal");
}

#[tokio::test]
async fn login_fails_with_wrong_password() {
    let (pool, _container) = start_migrated_postgres().await;
    let validator = InternalValidator::new(TEST_PRIVATE_KEY_PEM, TEST_PUBLIC_KEY_PEM)
        .expect("validator should build from valid PEM keys");

    validator
        .create_internal_user(&pool, "carol@example.com", "Carol", "correct horse battery staple")
        .await
        .expect("user creation should succeed");

    let result = validator.login(&pool, "carol@example.com", "wrong password").await;

    assert!(matches!(result, Err(AuthError::InvalidCredentials)));
}

#[tokio::test]
async fn login_fails_for_unknown_email() {
    let (pool, _container) = start_migrated_postgres().await;
    let validator = InternalValidator::new(TEST_PRIVATE_KEY_PEM, TEST_PUBLIC_KEY_PEM)
        .expect("validator should build from valid PEM keys");

    let result = validator.login(&pool, "nobody@example.com", "irrelevant").await;

    assert!(matches!(result, Err(AuthError::InvalidCredentials)));
}

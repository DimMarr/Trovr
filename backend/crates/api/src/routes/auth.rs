use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use trovr_auth::InternalValidator;

use crate::dto::UserResponse;
use crate::extract::CurrentUser;
use crate::{ApiError, AppState};

const MAX_EMAIL_BYTES: usize = 254;
const MIN_PASSWORD_CHARS: usize = 8;
const MAX_PASSWORD_CHARS: usize = 1024;
const MAX_DISPLAY_NAME_CHARS: usize = 100;

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterRequest {
    email: String,
    password: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TokenResponse {
    access_token: String,
    token_type: &'static str,
}

pub(crate) async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let internal = internal_auth(&state)?;
    if !state.settings.allow_registration {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "registration_disabled",
            "self-service registration is disabled",
        ));
    }

    let email = normalize_email(&body.email)?;
    let display_name = normalize_display_name(&body.display_name)?;
    validate_password(&body.password)?;

    let user_id = internal
        .create_internal_user(&state.pool, &email, &display_name, &body.password)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(UserResponse {
            id: user_id,
            email,
            display_name,
            issuer: "internal".to_string(),
        }),
    ))
}

pub(crate) async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    let internal = internal_auth(&state)?;
    let access_token = internal
        .login(&state.pool, &body.email, &body.password)
        .await?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer",
    }))
}

pub(crate) async fn me(CurrentUser(user): CurrentUser) -> Json<UserResponse> {
    Json(user.into())
}

/// Internal accounts only exist when internal auth is enabled.
fn internal_auth(state: &AppState) -> Result<&InternalValidator, ApiError> {
    state
        .internal_auth
        .as_deref()
        .ok_or_else(ApiError::not_found)
}

fn normalize_email(email: &str) -> Result<String, ApiError> {
    let email = email.trim().to_lowercase();
    let valid = email.len() <= MAX_EMAIL_BYTES
        && !email.chars().any(|c| c.is_whitespace() || c.is_control())
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && !domain.is_empty() && !domain.contains('@')
        });
    if valid {
        Ok(email)
    } else {
        Err(ApiError::invalid_request("email must be a valid address"))
    }
}

fn validate_password(password: &str) -> Result<(), ApiError> {
    let chars = password.chars().count();
    if chars < MIN_PASSWORD_CHARS {
        return Err(ApiError::invalid_request(
            "password must be at least 8 characters",
        ));
    }
    if chars > MAX_PASSWORD_CHARS {
        return Err(ApiError::invalid_request(
            "password must be at most 1024 characters",
        ));
    }
    Ok(())
}

fn normalize_display_name(display_name: &str) -> Result<String, ApiError> {
    let display_name = display_name.trim();
    if display_name.is_empty()
        || display_name.chars().count() > MAX_DISPLAY_NAME_CHARS
        || display_name.chars().any(char::is_control)
    {
        return Err(ApiError::invalid_request(
            "display name must be 1 to 100 characters without control characters",
        ));
    }
    Ok(display_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_are_trimmed_lowercased_and_checked() {
        assert_eq!(
            normalize_email(" Ann@Example.COM ").unwrap(),
            "ann@example.com"
        );
        for invalid in [
            "",
            "ann",
            "@example.com",
            "ann@",
            "a b@example.com",
            "a@b@c",
        ] {
            assert!(normalize_email(invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn passwords_need_eight_to_1024_characters() {
        assert!(validate_password("1234567").is_err());
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password(&"x".repeat(1024)).is_ok());
        assert!(validate_password(&"x".repeat(1025)).is_err());
    }

    #[test]
    fn display_names_are_trimmed_and_bounded() {
        assert_eq!(normalize_display_name("  Ann  ").unwrap(), "Ann");
        assert!(normalize_display_name("   ").is_err());
        assert!(normalize_display_name("Ann\nB").is_err());
        assert!(normalize_display_name(&"x".repeat(101)).is_err());
    }
}

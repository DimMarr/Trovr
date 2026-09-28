use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use trovr_auth::{AuthenticatedUser, TokenValidator};

use crate::{ApiError, AppState};

/// The caller, authenticated from an `Authorization: Bearer <token>` header
/// by whichever identity providers are enabled.
pub(crate) struct CurrentUser(pub AuthenticatedUser);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts).ok_or_else(ApiError::unauthorized)?;
        authenticate(state, token).await.map(CurrentUser)
    }
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    let value = parts.headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

/// Tries the internal validator first: it is local and cheap, and tokens from
/// an external provider fail it immediately (different issuer and key).
async fn authenticate(state: &AppState, token: &str) -> Result<AuthenticatedUser, ApiError> {
    if let Some(internal) = &state.internal_auth {
        match internal.validate(token).await {
            Ok(user) => return Ok(user),
            Err(err) if state.oidc_auth.is_none() => return Err(err.into()),
            Err(_) => {}
        }
    }

    match &state.oidc_auth {
        Some(oidc) => oidc.validate(token).await.map_err(ApiError::from),
        None => Err(ApiError::unauthorized()),
    }
}

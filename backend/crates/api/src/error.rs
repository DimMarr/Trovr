use std::fmt::Display;

use axum::Json;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use trovr_auth::AuthError;
use trovr_metadata::MetadataError;
use trovr_storage::StorageError;

/// An error answered as `{"error": {"code": ..., "message": ...}}`.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "resource not found")
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "missing or invalid access token",
        )
    }

    pub fn forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "you do not have enough access to this resource",
        )
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_request", message)
    }

    /// Logs the real cause and answers a generic 500.
    pub fn internal(err: impl Display) -> Self {
        tracing::error!(error = %err, "internal error");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "internal server error",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": { "code": self.code, "message": self.message } }));
        if self.status == StatusCode::UNAUTHORIZED {
            (self.status, [(header::WWW_AUTHENTICATE, "Bearer")], body).into_response()
        } else {
            (self.status, body).into_response()
        }
    }
}

impl From<MetadataError> for ApiError {
    fn from(err: MetadataError) -> Self {
        let (status, code) = match &err {
            MetadataError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            MetadataError::NameConflict => (StatusCode::CONFLICT, "name_conflict"),
            MetadataError::InvalidName(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_name"),
            MetadataError::InvalidContent(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "invalid_content")
            }
            MetadataError::NotAFolder => (StatusCode::UNPROCESSABLE_ENTITY, "not_a_folder"),
            MetadataError::NotAFile => (StatusCode::UNPROCESSABLE_ENTITY, "not_a_file"),
            MetadataError::CycleDetected => (StatusCode::CONFLICT, "cycle_detected"),
            MetadataError::NotTrashed => (StatusCode::CONFLICT, "not_trashed"),
            MetadataError::ParentTrashed => (StatusCode::CONFLICT, "parent_trashed"),
            MetadataError::StorageKeyInUse => (StatusCode::CONFLICT, "upload_already_used"),
            MetadataError::Database(_) => return Self::internal(&err),
        };
        Self::new(status, code, err.to_string())
    }
}

impl From<AuthError> for ApiError {
    fn from(err: AuthError) -> Self {
        match &err {
            AuthError::InvalidCredentials => Self::new(
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "invalid credentials",
            ),
            AuthError::InvalidToken(_) | AuthError::Token(_) => Self::unauthorized(),
            AuthError::UserAlreadyExists => Self::new(
                StatusCode::CONFLICT,
                "email_taken",
                "a user with this email already exists",
            ),
            AuthError::Discovery(detail) => {
                tracing::warn!(error = %detail, "identity provider unreachable");
                Self::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "identity_provider_unavailable",
                    "the identity provider is unavailable",
                )
            }
            AuthError::Database(_) | AuthError::Hash(_) => Self::internal(&err),
        }
    }
}

impl From<StorageError> for ApiError {
    fn from(err: StorageError) -> Self {
        match &err {
            StorageError::Request(_) => {
                tracing::error!(error = %err, "object storage request failed");
                Self::new(
                    StatusCode::BAD_GATEWAY,
                    "storage_unavailable",
                    "the storage backend is unavailable",
                )
            }
            StorageError::InvalidExpiry(_) => Self::internal(&err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn render(err: ApiError) -> (StatusCode, serde_json::Value) {
        let response = err.into_response();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn domain_errors_map_to_their_status_and_code() {
        let (status, body) = render(MetadataError::NameConflict.into()).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"]["code"], "name_conflict");

        let (status, body) = render(StorageError::Request("boom".to_string()).into()).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body["error"]["code"], "storage_unavailable");
    }

    #[tokio::test]
    async fn internal_errors_hide_their_cause() {
        let (status, body) =
            render(MetadataError::Database(sqlx::Error::PoolTimedOut).into()).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"]["message"], "internal server error");
    }
}

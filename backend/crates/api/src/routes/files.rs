use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use trovr_auth::AuthenticatedUser;
use trovr_metadata::{NewContent, NewFile, Node, Role};
use trovr_storage::{ObjectInfo, ObjectStorage};
use uuid::Uuid;

use crate::access::authorize;
use crate::dto::{NodeResponse, PresignedResponse, VersionResponse};
use crate::extract::CurrentUser;
use crate::{ApiError, AppState};

const MAX_MIME_TYPE_BYTES: usize = 255;
const FALLBACK_MIME_TYPE: &str = "application/octet-stream";

#[derive(Debug, Deserialize)]
pub(crate) struct CreateUploadRequest {
    size_bytes: i64,
    mime_type: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct UploadResponse {
    storage_key: String,
    upload: PresignedResponse,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateFileRequest {
    storage_key: String,
    #[serde(default)]
    parent_id: Option<Uuid>,
    name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateVersionRequest {
    storage_key: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DownloadQuery {
    version_id: Option<Uuid>,
}

/// Step 1 of an upload: presign a PUT the browser sends straight to storage.
pub(crate) async fn create_upload(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<CreateUploadRequest>,
) -> Result<(StatusCode, Json<UploadResponse>), ApiError> {
    if body.size_bytes < 0 {
        return Err(ApiError::invalid_request("size_bytes must not be negative"));
    }
    if body.size_bytes > state.settings.max_upload_bytes {
        return Err(too_large(state.settings.max_upload_bytes));
    }
    validate_mime_type(&body.mime_type)?;

    let storage_key = ObjectStorage::new_object_key(user.user_id);
    let request = state
        .storage
        .presign_upload(&storage_key, &body.mime_type, state.settings.upload_url_ttl)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse {
            storage_key,
            upload: PresignedResponse::new(request, state.settings.upload_url_ttl),
        }),
    ))
}

/// Step 2 of an upload: record the uploaded object as a new file.
pub(crate) async fn create_file(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<CreateFileRequest>,
) -> Result<(StatusCode, Json<NodeResponse>), ApiError> {
    if let Some(parent_id) = body.parent_id {
        authorize(&state, &user, parent_id, Role::Editor).await?;
    }
    let object = uploaded_object(&state, &user, &body.storage_key).await?;

    let node = state
        .nodes
        .create_file(NewFile {
            owner_id: user.user_id,
            parent_id: body.parent_id,
            name: body.name,
            mime_type: object
                .content_type
                .unwrap_or_else(|| FALLBACK_MIME_TYPE.to_string()),
            content: NewContent {
                storage_key: body.storage_key,
                size_bytes: object.size_bytes,
                checksum_sha256: None,
            },
        })
        .await?;

    Ok((StatusCode::CREATED, Json(node.into())))
}

/// Step 2 of an upload, alternatively: record the object as a file's new
/// current version.
pub(crate) async fn create_version(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
    Json(body): Json<CreateVersionRequest>,
) -> Result<(StatusCode, Json<VersionResponse>), ApiError> {
    authorize(&state, &user, node_id, Role::Editor).await?;
    let object = uploaded_object(&state, &user, &body.storage_key).await?;

    let version = state
        .nodes
        .add_version(
            node_id,
            user.user_id,
            NewContent {
                storage_key: body.storage_key,
                size_bytes: object.size_bytes,
                checksum_sha256: None,
            },
        )
        .await?;

    Ok((StatusCode::CREATED, Json(version.into())))
}

pub(crate) async fn list_versions(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
) -> Result<Json<Vec<VersionResponse>>, ApiError> {
    authorize(&state, &user, node_id, Role::Viewer).await?;
    let versions = state.nodes.list_versions(node_id).await?;
    Ok(Json(
        versions.into_iter().map(VersionResponse::from).collect(),
    ))
}

/// Presigns a GET for the current version, or for `?version_id=`.
pub(crate) async fn download(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(node_id): Path<Uuid>,
    Query(query): Query<DownloadQuery>,
) -> Result<Json<PresignedResponse>, ApiError> {
    let (node, _) = authorize(&state, &user, node_id, Role::Viewer).await?;
    Ok(Json(
        presign_download(&state, &node, query.version_id).await?,
    ))
}

/// Presigns a GET for a version of a live file (the current one by default),
/// saved under the file's name.
pub(crate) async fn presign_download(
    state: &AppState,
    node: &Node,
    version_id: Option<Uuid>,
) -> Result<PresignedResponse, ApiError> {
    let version = match version_id {
        Some(version_id) => state
            .nodes
            .list_versions(node.id)
            .await?
            .into_iter()
            .find(|version| version.id == version_id)
            .ok_or_else(ApiError::not_found)?,
        None => state.nodes.current_version(node.id).await?,
    };

    let request = state
        .storage
        .presign_download(
            &version.storage_key,
            &node.name,
            state.settings.download_url_ttl,
        )
        .await?;

    Ok(PresignedResponse::new(
        request,
        state.settings.download_url_ttl,
    ))
}

/// Checks that `storage_key` is one of the caller's uploads, that it really
/// was uploaded, and that it fits the size limit (deleting it otherwise).
async fn uploaded_object(
    state: &AppState,
    user: &AuthenticatedUser,
    storage_key: &str,
) -> Result<ObjectInfo, ApiError> {
    if !ObjectStorage::key_belongs_to(storage_key, user.user_id) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_upload",
            "unknown upload",
        ));
    }

    let object = state.storage.head(storage_key).await?.ok_or_else(|| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "upload_missing",
            "nothing was uploaded for this storage key",
        )
    })?;

    if object.size_bytes > state.settings.max_upload_bytes {
        if let Err(err) = state.storage.delete(&[storage_key.to_string()]).await {
            tracing::warn!(error = %err, storage_key, "failed to delete an oversized upload");
        }
        return Err(too_large(state.settings.max_upload_bytes));
    }

    Ok(object)
}

fn too_large(max_upload_bytes: i64) -> ApiError {
    ApiError::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "upload_too_large",
        format!("uploads are limited to {max_upload_bytes} bytes"),
    )
}

/// The media type is signed into the presigned PUT and sent back as a
/// header, so it must be a plain printable ASCII value.
fn validate_mime_type(mime_type: &str) -> Result<(), ApiError> {
    let valid = !mime_type.trim().is_empty()
        && mime_type.len() <= MAX_MIME_TYPE_BYTES
        && mime_type.chars().all(|c| c == ' ' || c.is_ascii_graphic());
    if valid {
        Ok(())
    } else {
        Err(ApiError::invalid_request(
            "mime_type must be a non-empty printable ASCII media type",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_types_must_be_printable_ascii() {
        assert!(validate_mime_type("text/plain; charset=utf-8").is_ok());
        for invalid in [
            "",
            "   ",
            "text/plain\r\nx: y",
            "image/pngé",
            &"a".repeat(256),
        ] {
            assert!(validate_mime_type(invalid).is_err(), "{invalid:?}");
        }
    }
}

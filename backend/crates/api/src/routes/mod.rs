use axum::Router;
use axum::routing::{get, post};

use crate::AppState;

pub(crate) mod auth;
pub(crate) mod files;
pub(crate) mod health;
pub(crate) mod nodes;

/// Routes served under `/api/v1`.
pub(crate) fn api() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/me", get(auth::me))
        .route("/nodes", get(nodes::list_root))
        .route("/folders", post(nodes::create_folder))
        .route("/nodes/{id}", get(nodes::get_node))
        .route("/nodes/{id}/children", get(nodes::list_children))
        .route("/nodes/{id}/path", get(nodes::get_path))
        .route("/nodes/{id}/rename", post(nodes::rename))
        .route("/nodes/{id}/move", post(nodes::move_node))
        .route("/uploads", post(files::create_upload))
        .route("/files", post(files::create_file))
        .route(
            "/nodes/{id}/versions",
            get(files::list_versions).post(files::create_version),
        )
        .route("/nodes/{id}/download", get(files::download))
}

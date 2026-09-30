use axum::Router;
use axum::routing::{delete, get, post};

use crate::AppState;

pub(crate) mod auth;
pub(crate) mod files;
pub(crate) mod health;
pub(crate) mod nodes;
pub(crate) mod public;
pub(crate) mod shares;
pub(crate) mod trash;

/// Routes served under `/api/v1`.
pub(crate) fn api() -> Router<AppState> {
    Router::new()
        .route("/auth/config", get(auth::config))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/me", get(auth::me))
        .route("/nodes", get(nodes::list_root))
        .route("/folders", post(nodes::create_folder))
        .route("/nodes/{id}", get(nodes::get_node).delete(trash::trash))
        .route("/nodes/{id}/children", get(nodes::list_children))
        .route("/nodes/{id}/path", get(nodes::get_path))
        .route("/nodes/{id}/rename", post(nodes::rename))
        .route("/nodes/{id}/move", post(nodes::move_node))
        .route("/nodes/{id}/restore", post(trash::restore))
        .route("/trash", get(trash::list))
        .route("/trash/{id}", delete(trash::purge))
        .route("/uploads", post(files::create_upload))
        .route("/files", post(files::create_file))
        .route(
            "/nodes/{id}/versions",
            get(files::list_versions).post(files::create_version),
        )
        .route("/nodes/{id}/download", get(files::download))
        .route("/nodes/{id}/shares", get(shares::list).post(shares::share))
        .route("/nodes/{id}/shares/{user_id}", delete(shares::unshare))
        .route("/shared", get(shares::shared_with_me))
        .route("/nodes/{id}/links", post(shares::create_link))
        .route("/nodes/{id}/links/{link_id}", delete(shares::delete_link))
        // Unauthenticated: the link token is the credential.
        .route("/public/{token}", get(public::root))
        .route(
            "/public/{token}/nodes/{id}/children",
            get(public::list_children),
        )
        .route("/public/{token}/nodes/{id}/download", get(public::download))
}

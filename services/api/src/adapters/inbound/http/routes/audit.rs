//! Routes audit logs + watched users + discord roles.

use axum::routing::{delete, get, patch, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Audit logs
        .route("/api/audit-logs", get(handlers::audit_logs::list_audit_logs).post(handlers::audit_logs::create_audit_log))
        .route("/api/audit-logs/{guild_id}", delete(handlers::audit_logs::purge_audit_logs))
        // Watched users
        .route("/api/watched-users", get(handlers::watched_users::list_watched_users).post(handlers::watched_users::add_watched_user))
        .route("/api/watched-users/{guild_id}/{user_id}", get(handlers::watched_users::get_user_dossier).delete(handlers::watched_users::remove_watched_user))
        // Discord roles (CRUD + sync)
        .route("/api/discord-roles/{guild_id}", get(handlers::discord_roles::list_roles))
        .route("/api/discord-roles/{guild_id}/sync", post(handlers::discord_roles::sync_roles))
        .route("/api/discord-roles/{guild_id}/create", post(handlers::discord_roles::create_role))
        .route("/api/discord-roles/{guild_id}/{role_id}", patch(handlers::discord_roles::edit_role).delete(handlers::discord_roles::delete_role))
}

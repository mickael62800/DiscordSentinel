//! Routes members (DB-backed + direct Discord API).

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn member_inner() -> Router<AppState> {
    Router::new()
        .route("/{guild_id}", get(handlers::guild_members::list_members_db))
        .route("/{guild_id}/{user_id}", get(handlers::guild_members::get_member).patch(handlers::guild_members::update_member).delete(handlers::guild_members::remove_member))
        .route("/{guild_id}/{user_id}/summary", get(handlers::guild_members::get_member_summary))
        .route("/{guild_id}/{user_id}/reset", post(handlers::guild_members::reset_member))
        .route("/sync", post(handlers::guild_members::sync_members))
        .route("/register", post(handlers::guild_members::register_member))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // Members (DB-backed)
        .nest("/api/members", member_inner())
        // Guild members (direct Discord API)
        .route("/api/guilds/{guild_id}/members", get(handlers::guild_members::list_members))
        // Guild text channels (direct Discord API, Phase 9 Part E)
        .route("/api/guilds/{guild_id}/channels", get(handlers::guild_channels::list_text_channels))
}

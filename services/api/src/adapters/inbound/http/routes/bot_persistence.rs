//! Routes bot persistence (endpoints fire-and-forget consommes par les bots).

use axum::routing::{delete, get, patch, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Bot persistence (fire-and-forget endpoints for bot data)
        .route("/api/name-history", post(handlers::bot_persistence::create_name_history))
        .route("/api/levels/{guild_id}/{user_id}/streak", patch(handlers::bot_persistence::update_streak))
        .route("/api/tickets/{id}/sla", patch(handlers::bot_persistence::update_ticket_sla))
        .route("/api/sponsorships", post(handlers::bot_persistence::create_sponsorship))
        .route("/api/sponsorships/{guild_id}", get(handlers::bot_persistence::list_sponsorships))
        .route("/api/temp-roles", post(handlers::bot_persistence::create_temp_role))
        .route("/api/temp-roles/{guild_id}", get(handlers::bot_persistence::list_temp_roles))
        .route("/api/temp-roles/{guild_id}/{user_id}/{role_id}", delete(handlers::bot_persistence::delete_temp_role))
        .route("/api/moderation/pending", post(handlers::bot_persistence::create_pending_action))
        .route("/api/moderation/pending/guild/{guild_id}", get(handlers::bot_persistence::list_pending_actions))
        .route("/api/moderation/pending/{id}/resolve", patch(handlers::bot_persistence::resolve_pending_action))
}

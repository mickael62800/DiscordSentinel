//! Routes progression: conduct, levels, role panels, auto-roles.

use axum::routing::{delete, get, patch, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn conduct_inner() -> Router<AppState> {
    Router::new()
        .route("/config/{guild_id}", get(handlers::conduct::get_config))
        .route("/config", post(handlers::conduct::save_config))
        .route("/{guild_id}/{user_id}", get(handlers::conduct::get_points))
        .route("/{guild_id}/leaderboard", get(handlers::conduct::get_leaderboard))
        .route("/{guild_id}/{user_id}/log", get(handlers::conduct::get_points_log))
        .route("/{guild_id}/{user_id}/add", post(handlers::conduct::add_points))
        // Endpoints appeles par moderation-worker : regen periodique +
        // creation des propositions de ban pour les users a 0 points
        // (cf. WORKERS_ARCHITECTURE_STATE.md P0 #1 + #2).
        .route("/regen-tick", post(handlers::conduct::run_regen_tick))
        .route("/sync-ban-proposals", post(handlers::conduct::sync_ban_proposals))
}

fn level_inner() -> Router<AppState> {
    Router::new()
        .route("/config/{guild_id}", get(handlers::levels::get_config))
        .route("/config", post(handlers::levels::save_config))
        .route("/xp", post(handlers::levels::add_xp))
        .route("/{guild_id}/{user_id}", get(handlers::levels::get_user_level))
        .route("/{guild_id}/leaderboard", get(handlers::levels::get_leaderboard))
        .route("/rewards/{guild_id}", get(handlers::levels::get_rewards))
        .route("/rewards", post(handlers::levels::set_reward))
        .route("/rewards/{guild_id}/{level}", delete(handlers::levels::delete_reward))
}

fn role_panel_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::role_panels::create_panel))
        .route("/{guild_id}", get(handlers::role_panels::list_panels))
        .route("/detail/{panel_id}", get(handlers::role_panels::get_panel).delete(handlers::role_panels::delete_panel))
        .route("/by-message/{message_id}", get(handlers::role_panels::get_panel_by_message))
        .route("/set-message", patch(handlers::role_panels::set_message_id))
}

fn auto_role_inner() -> Router<AppState> {
    Router::new()
        .route("/{guild_id}", get(handlers::role_panels::list_auto_roles))
        .route("/", post(handlers::role_panels::add_auto_role))
        .route("/{guild_id}/{role_id}", delete(handlers::role_panels::delete_auto_role))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/api/conduct", conduct_inner())
        .nest("/api/levels", level_inner())
        .nest("/api/role-panels", role_panel_inner())
        .nest("/api/auto-roles", auto_role_inner())
}

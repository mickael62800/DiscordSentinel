//! Routes moderation + strikes + notes + reminders.

use axum::routing::{delete, get, patch, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn moderation_inner() -> Router<AppState> {
    Router::new()
        .route("/actions", post(handlers::moderation::log_action))
        .route("/actions/{id}", delete(handlers::moderation::delete_action))
        .route("/bans", get(handlers::moderation::list_bans))
        .route("/execute-ban", post(handlers::moderation::execute_ban))
        .route("/execute-unban", post(handlers::moderation::execute_unban))
        .route("/execute-mute", post(handlers::moderation::execute_mute))
        .route("/history/{guild_id}/{user_id}", get(handlers::moderation::get_history))
        .route("/modstats/{guild_id}", get(handlers::moderation::get_modstats))
        .route("/evidence", post(handlers::moderation::add_evidence))
        .route("/evidence/{action_id}", get(handlers::moderation::list_evidence))
        .route("/review", post(handlers::moderation::add_review))
        .route("/review/{guild_id}/pending", get(handlers::moderation::list_pending_reviews))
        .route("/review/{id}/resolve", patch(handlers::moderation::resolve_review))
}

fn strikes_inner() -> Router<AppState> {
    Router::new()
        .route("/config/{guild_id}", get(handlers::strikes::get_config).put(handlers::strikes::save_config))
        .route("/{guild_id}/{user_id}", get(handlers::strikes::get_active_strikes).delete(handlers::strikes::reset_strikes))
        .route("/", post(handlers::strikes::add_strike))
}

fn notes_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::notes::add_note))
        .route("/{guild_id}/{user_id}", get(handlers::notes::get_notes))
        .route("/{id}", delete(handlers::notes::delete_note))
}

fn reminders_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::reminders::create_reminder))
        .route("/pending", get(handlers::reminders::get_pending))
        .route("/{guild_id}", get(handlers::reminders::list_by_guild))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/api/moderation", moderation_inner())
        .nest("/api/strikes", strikes_inner())
        .nest("/api/notes", notes_inner())
        .nest("/api/reminders", reminders_inner())
}

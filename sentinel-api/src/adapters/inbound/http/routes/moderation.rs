//! Routes moderation + strikes + notes + reminders.

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn moderation_inner() -> Router<AppState> {
    Router::new()
        .route("/actions", post(handlers::moderation::actions::log_action))
        .route("/actions/{id}", delete(handlers::moderation::actions::delete_action))
        .route("/bans", get(handlers::moderation::actions::list_bans))
        .route("/execute-ban", post(handlers::moderation::actions::execute_ban))
        .route("/execute-unban", post(handlers::moderation::actions::execute_unban))
        .route("/execute-mute", post(handlers::moderation::actions::execute_mute))
        .route("/history/{guild_id}/{user_id}", get(handlers::moderation::actions::get_history))
        .route("/modstats/{guild_id}", get(handlers::moderation::actions::get_modstats))
        .route("/modstats/{guild_id}/trend", get(handlers::moderation::actions::get_modstats_trend))
        .route("/evidence", post(handlers::moderation::actions::add_evidence))
        .route("/evidence/{action_id}", get(handlers::moderation::actions::list_evidence))
        .route("/review", post(handlers::moderation::actions::add_review))
        .route("/review/{guild_id}/pending", get(handlers::moderation::actions::list_pending_reviews))
        .route("/review/{id}/resolve", patch(handlers::moderation::actions::resolve_review))
}

fn strikes_inner() -> Router<AppState> {
    Router::new()
        .route("/config/{guild_id}", get(handlers::moderation::strikes::get_config).put(handlers::moderation::strikes::save_config))
        .route("/{guild_id}/{user_id}", get(handlers::moderation::strikes::get_active_strikes).delete(handlers::moderation::strikes::reset_strikes))
        .route("/", post(handlers::moderation::strikes::add_strike))
}

fn notes_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::moderation::notes::add_note))
        .route("/{guild_id}/{user_id}", get(handlers::moderation::notes::get_notes))
        .route("/{id}", delete(handlers::moderation::notes::delete_note))
}

fn reminders_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::moderation::reminders::create_reminder))
        .route("/pending", get(handlers::moderation::reminders::get_pending))
        .route("/{guild_id}", get(handlers::moderation::reminders::list_by_guild))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/api/moderation", moderation_inner())
        .nest("/api/strikes", strikes_inner())
        .nest("/api/notes", notes_inner())
        .nest("/api/reminders", reminders_inner())
}

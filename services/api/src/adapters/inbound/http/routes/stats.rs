//! Routes stats (messages / voice / leaderboards).

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn stats_inner() -> Router<AppState> {
    Router::new()
        .route("/messages", post(handlers::stats::record_messages))
        .route("/voice", post(handlers::stats::record_voice))
        .route("/{guild_id}/user/{user_id}", get(handlers::stats::get_user_stats))
        .route("/{guild_id}/overview", get(handlers::stats::get_guild_overview))
        .route("/{guild_id}/leaderboard", get(handlers::stats::get_leaderboard))
        .route("/{guild_id}/voice-stats", get(handlers::stats::get_guild_voice_stats))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/api/stats", stats_inner())
        // Dashboard stats (hors du nest /api pour eviter le conflit avec /api/stats)
        .route("/api/stats", get(handlers::dashboard::get_dashboard_stats))
}

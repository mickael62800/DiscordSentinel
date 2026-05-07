//! Routes analytics (inference lourde, rate limit strict).
//! Montees sous `/api/analytics`.

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn inner() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::audit::analytics::get_full_analytics))
        .route("/heatmap", get(handlers::audit::analytics::get_heatmap))
        .route("/actions", get(handlers::audit::analytics::get_action_distribution))
        .route("/top-infractors", get(handlers::audit::analytics::get_top_infractors))
        .route("/moderation-trend", get(handlers::audit::analytics::get_moderation_trend))
        .route("/peak-hours", get(handlers::audit::analytics::get_peak_hours))
        .route("/reset", post(handlers::audit::analytics::reset_analytics))
        // Jobs declenches par sentinel-worker.
        .route("/snapshot/daily", post(handlers::audit::snapshots::snapshot_daily_all))
        .route("/snapshot/hourly", post(handlers::audit::snapshots::snapshot_hourly_all))
        .route("/retention-cleanup", post(handlers::audit::snapshots::retention_cleanup_all))
        .route("/publish-top-users", post(handlers::audit::snapshots::publish_top_users_all))
        // Export user-facing.
        .route("/export", get(handlers::audit::snapshots::export_analytics))
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/analytics", inner())
}

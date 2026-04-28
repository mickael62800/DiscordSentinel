//! Routes analytics (inference lourde, rate limit strict).
//! Montees sous `/api/analytics`.

use axum::routing::get;
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
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/analytics", inner())
}

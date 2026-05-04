//! Routes security (montees sous `/api/security`).

use axum::routing::delete;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn security_inner() -> Router<AppState> {
    Router::new()
        .route("/events", post(handlers::audit::security::report_event).get(handlers::audit::security::list_events))
        .route("/events/{guild_id}", delete(handlers::audit::security::purge_events))
        // Phase 5F — quarantaine (timer kick deplace dans sentinel-worker).
        .route(
            "/quarantine",
            post(handlers::system::quarantine::create_quarantine),
        )
        .route(
            "/quarantine/{guild_id}/{user_id}",
            delete(handlers::system::quarantine::delete_quarantine),
        )
        // Phase 5G — lockdown auto-revert (worker).
        .route(
            "/lockdown",
            post(handlers::system::lockdown::create_lockdown),
        )
        .route(
            "/lockdown/{guild_id}",
            delete(handlers::system::lockdown::delete_lockdown),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/security", security_inner())
}

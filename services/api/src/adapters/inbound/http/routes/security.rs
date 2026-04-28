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
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/security", security_inner())
}

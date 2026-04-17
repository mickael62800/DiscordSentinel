//! Routes security (montees sous `/api/security`).

use axum::routing::{delete, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn security_inner() -> Router<AppState> {
    Router::new()
        .route("/events", post(handlers::security::report_event).get(handlers::security::list_events))
        .route("/events/{guild_id}", delete(handlers::security::purge_events))
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/security", security_inner())
}

//! Routes automod (montees sous `/api/automod`).
//! Phase 4 — page web /automod : timeline des detections.

use axum::routing::get;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn automod_inner() -> Router<AppState> {
    Router::new().route(
        "/{guild_id}/detections",
        get(handlers::automod::list_detections),
    )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/automod", automod_inner())
}

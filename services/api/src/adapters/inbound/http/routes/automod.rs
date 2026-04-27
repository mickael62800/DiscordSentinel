//! Routes automod (montees sous `/api/automod`).
//! Phase 4 — page web /automod : timeline des detections.
//! Phase Sync — review cards (sync Discord <-> web).

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn automod_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}/detections",
            get(handlers::automod::list_detections),
        )
        .route(
            "/{guild_id}/reviews",
            get(handlers::automod::list_reviews),
        )
        .route(
            "/reviews",
            post(handlers::automod::create_review),
        )
        .route(
            "/reviews/{review_id}/resolve",
            post(handlers::automod::resolve_review),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/automod", automod_inner())
}

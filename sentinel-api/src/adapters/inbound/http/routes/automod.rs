//! Routes automod (montees sous `/api/automod`).
//! Phase 4 — page web /automod : timeline des detections.
//! Phase Sync — review cards (sync Discord <-> web).

use axum::routing::get;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn automod_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}/detections",
            get(handlers::moderation::automod::list_detections),
        )
        .route(
            "/{guild_id}/reviews",
            get(handlers::moderation::automod::list_reviews),
        )
        .route(
            "/reviews",
            post(handlers::moderation::automod::create_review),
        )
        .route(
            "/reviews/{review_id}/resolve",
            post(handlers::moderation::automod::resolve_review),
        )
        .route(
            "/reviews/{review_id}/vote",
            post(handlers::moderation::automod::vote_review),
        )
        .route(
            "/reviews/{review_id}/votes",
            get(handlers::moderation::automod::list_review_votes),
        )
        .route(
            "/reviews/{review_id}/decide",
            post(handlers::moderation::automod::decide_review),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/automod", automod_inner())
}

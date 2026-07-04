//! Routes du jeu Influence.

use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn influence_inner() -> Router<AppState> {
    Router::new().route(
        "/{guild_id}/profile",
        post(handlers::influence::citizens::view_profile),
    )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/influence", influence_inner())
}

//! Routes de l'administrateur tournant (sous `/api/rotation`).

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}",
            get(handlers::system::admin_rotation::get_state),
        )
        .route(
            "/{guild_id}/save",
            post(handlers::system::admin_rotation::save_state),
        )
        .route(
            "/{guild_id}/served",
            post(handlers::system::admin_rotation::record_served),
        )
        .route(
            "/{guild_id}/history",
            get(handlers::system::admin_rotation::history),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/rotation", inner())
}

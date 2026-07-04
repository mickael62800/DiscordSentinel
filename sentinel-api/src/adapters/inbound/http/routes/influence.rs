//! Routes du jeu Influence.

use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn influence_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}/profile",
            post(handlers::influence::citizens::view_profile),
        )
        .route(
            "/{guild_id}/orgs",
            post(handlers::influence::orgs::create_org),
        )
        .route(
            "/{guild_id}/orgs/info",
            post(handlers::influence::orgs::org_info),
        )
        .route(
            "/{guild_id}/orgs/join",
            post(handlers::influence::orgs::join_org),
        )
        .route(
            "/{guild_id}/orgs/members",
            post(handlers::influence::orgs::org_members),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/influence", influence_inner())
}

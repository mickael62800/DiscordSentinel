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
        .route(
            "/{guild_id}/motions",
            post(handlers::influence::votes::create_motion),
        )
        .route(
            "/{guild_id}/motions/vote",
            post(handlers::influence::votes::cast_vote),
        )
        .route(
            "/{guild_id}/motions/close",
            post(handlers::influence::votes::close_motion),
        )
        .route(
            "/{guild_id}/motions/state",
            post(handlers::influence::votes::motion_state),
        )
        .route(
            "/{guild_id}/capital",
            post(handlers::influence::capital::view_capital),
        )
        .route(
            "/{guild_id}/capital/convert",
            post(handlers::influence::capital::convert_capital),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/influence", influence_inner())
}

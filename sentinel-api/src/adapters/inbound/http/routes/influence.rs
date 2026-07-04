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
        .route(
            "/{guild_id}/laws",
            post(handlers::influence::laws::propose_law),
        )
        .route(
            "/{guild_id}/laws/vote",
            post(handlers::influence::laws::law_vote),
        )
        .route(
            "/{guild_id}/laws/state",
            post(handlers::influence::laws::law_state),
        )
        .route(
            "/{guild_id}/laws/message",
            post(handlers::influence::laws::set_law_message),
        )
        .route(
            "/internal/jobs/close-laws",
            post(handlers::influence::laws::job_close_laws),
        )
        .route(
            "/{guild_id}/investigations",
            post(handlers::influence::information::open_investigation),
        )
        .route(
            "/{guild_id}/intel",
            post(handlers::influence::information::list_intel),
        )
        .route(
            "/{guild_id}/reveal",
            post(handlers::influence::information::reveal),
        )
        .route(
            "/internal/jobs/resolve-investigations",
            post(handlers::influence::information::job_resolve_investigations),
        )
        .route(
            "/{guild_id}/orgs/relation",
            post(handlers::influence::orgs::set_relation),
        )
        .route(
            "/{guild_id}/archives",
            post(handlers::influence::archives::list_archives),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/influence", influence_inner())
}

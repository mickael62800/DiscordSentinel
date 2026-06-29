//! Routes Bump rewards (montees sous `/api/bump`).

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn inner() -> Router<AppState> {
    Router::new()
        .route(
            "/due-reminders",
            get(handlers::community::bump::due_reminders),
        )
        .route(
            "/{guild_id}/reminder-sent",
            post(handlers::community::bump::mark_reminder_sent),
        )
        .route(
            "/{guild_id}/{user_id}",
            post(handlers::community::bump::record_bump),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/bump", inner())
}

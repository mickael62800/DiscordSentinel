//! Routes de sauvegarde / restauration de serveur (montees sous
//! `/api/guild-backup`).

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}/snapshots",
            post(handlers::guild_backup::snapshots::store_snapshot)
                .get(handlers::guild_backup::snapshots::list_snapshots),
        )
        .route(
            "/snapshots/{snapshot_id}",
            get(handlers::guild_backup::snapshots::get_snapshot)
                .delete(handlers::guild_backup::snapshots::delete_snapshot),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/guild-backup", inner())
}

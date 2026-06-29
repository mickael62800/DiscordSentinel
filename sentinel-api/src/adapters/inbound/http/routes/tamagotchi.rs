//! Routes du jeu Tamagotchi (montees sous `/api/tamagotchi`).

use axum::routing::{delete, get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn inner() -> Router<AppState> {
    Router::new()
        // Worker : tick de cycle de vie (decroissance/maladie/mort).
        .route("/tick", post(handlers::tamagotchi::pets::tick_all))
        // Admin web : liste des compagnons de la guild + suppression.
        // (Toutes les interactions du bot passent par le TamagotchiService gRPC.)
        .route(
            "/{guild_id}/pets",
            get(handlers::tamagotchi::pets::list_pets),
        )
        .route(
            "/{guild_id}/pets/{pet_id}",
            delete(handlers::tamagotchi::pets::delete_pet),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/tamagotchi", inner())
}

//! Routes du jeu Tamagotchi (montees sous `/api/tamagotchi`).

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn inner() -> Router<AppState> {
    Router::new()
        .route("/pets", post(handlers::tamagotchi::pets::create_pet))
        .route("/pets/{pet_id}/care", post(handlers::tamagotchi::pets::care_pet))
        .route("/pets/{pet_id}/train", post(handlers::tamagotchi::pets::train_pet))
        .route("/visit", post(handlers::tamagotchi::pets::visit))
        .route("/tick", post(handlers::tamagotchi::pets::tick_all))
        .route("/{guild_id}/{owner_id}", get(handlers::tamagotchi::pets::get_pet))
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/tamagotchi", inner())
}

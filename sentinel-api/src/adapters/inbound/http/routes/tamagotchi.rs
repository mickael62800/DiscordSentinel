//! Routes du jeu Tamagotchi (montees sous `/api/tamagotchi`).

use axum::routing::{delete, get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn inner() -> Router<AppState> {
    Router::new()
        .route("/pets", post(handlers::tamagotchi::pets::create_pet))
        .route("/pets/{pet_id}/care", post(handlers::tamagotchi::pets::care_pet))
        .route("/pets/{pet_id}/train", post(handlers::tamagotchi::pets::train_pet))
        .route("/visit", post(handlers::tamagotchi::pets::visit))
        .route("/combat", post(handlers::tamagotchi::pets::combat))
        .route("/tick", post(handlers::tamagotchi::pets::tick_all))
        .route("/cards", get(handlers::tamagotchi::pets::list_cards))
        .route("/{guild_id}/{owner_id}/card", post(handlers::tamagotchi::pets::set_card_location))
        // Admin web : liste des compagnons de la guild + suppression.
        .route("/{guild_id}/pets", get(handlers::tamagotchi::pets::list_pets))
        .route("/{guild_id}/pets/{pet_id}", delete(handlers::tamagotchi::pets::delete_pet))
        .route("/{guild_id}/{owner_id}", get(handlers::tamagotchi::pets::get_pet))
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/tamagotchi", inner())
}

//! Routes games (abonnements / alertes).

use axum::routing::{delete, get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn game_inner() -> Router<AppState> {
    Router::new()
        .route("/{guild_id}", get(handlers::games::list_games))
        .route("/", post(handlers::games::create_game))
        .route("/{guild_id}/{game_id}", delete(handlers::games::delete_game))
        .route("/{guild_id}/{game_id}/subscribe", post(handlers::games::subscribe))
        .route("/{guild_id}/{game_id}/subscribe/{user_id}", delete(handlers::games::unsubscribe))
        .route("/{guild_id}/{game_id}/subscribers", get(handlers::games::get_subscribers))
        .route("/{guild_id}/by-name/{game_name}", get(handlers::games::get_game_by_name))
        .route("/{guild_id}/user/{user_id}", get(handlers::games::get_user_games))
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/games", game_inner())
}

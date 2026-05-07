//! Routes games (gestion des jeux + panels Discord).

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn game_inner() -> Router<AppState> {
    Router::new()
        .route("/{guild_id}", get(handlers::casino::games::list_games))
        .route("/", post(handlers::casino::games::create_game))
        .route("/{guild_id}/{game_id}", delete(handlers::casino::games::delete_game).patch(handlers::casino::games::update_game))
        .route("/{guild_id}/{game_id}/role", patch(handlers::casino::games::set_role_id))
        .route("/{guild_id}/upload-emoji", post(handlers::casino::games::upload_emoji))
        .route("/{guild_id}/by-name/{game_name}", get(handlers::casino::games::get_game_by_name))
        .route("/{guild_id}/by-category", get(handlers::casino::games::list_games_by_category))
        .route("/{guild_id}/panels", post(handlers::casino::games::save_panel).get(handlers::casino::games::list_panels))
        .route("/{guild_id}/panels/by-message/{message_id}", get(handlers::casino::games::find_panel_by_message))
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/games", game_inner())
}

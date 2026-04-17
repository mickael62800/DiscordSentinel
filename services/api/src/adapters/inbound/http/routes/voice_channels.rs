//! Routes voice channels (salons vocaux dynamiques, whitelists, bans, invites, themes).

use axum::routing::{delete, get, patch, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn voice_inner() -> Router<AppState> {
    Router::new()
        .route("/_all", get(handlers::voice_channels::list_all_channels))
        .route("/{guild_id}", get(handlers::voice_channels::list_channels))
        .route(
            "/{guild_id}/history",
            get(handlers::voice_channels::list_history_channels)
                .delete(handlers::voice_channels::purge_history),
        )
        .route("/", post(handlers::voice_channels::create_channel))
        .route(
            "/by-channel/{channel_id}",
            get(handlers::voice_channels::get_channel_detail)
                .patch(handlers::voice_channels::update_channel)
                .delete(handlers::voice_channels::delete_channel),
        )
        .route("/by-channel/{channel_id}/close", patch(handlers::voice_channels::close_channel))
        .route("/by-channel/{channel_id}/events", get(handlers::voice_channels::list_channel_events))
        .route("/by-channel/{channel_id}/purge", delete(handlers::voice_channels::purge_channel))
        .route("/by-channel/{channel_id}/transfer", patch(handlers::voice_channels::transfer_ownership))
        .route("/by-channel/{channel_id}/co-admins", post(handlers::voice_channels::add_co_admin))
        .route("/by-channel/{channel_id}/co-admins/{user_id}", delete(handlers::voice_channels::remove_co_admin))
        .route("/whitelist/{guild_id}/{owner_id}", get(handlers::voice_channels::get_whitelist))
        .route("/whitelist", post(handlers::voice_channels::add_to_whitelist))
        .route("/whitelist/{guild_id}/{owner_id}/{target_id}", delete(handlers::voice_channels::remove_from_whitelist))
        .route("/by-channel/{channel_id}/bans", post(handlers::voice_channels::ban_from_channel))
        .route(
            "/by-channel/{channel_id}/bans/{user_id}",
            delete(handlers::voice_channels::unban_from_channel)
                .get(handlers::voice_channels::check_ban),
        )
        // Invite Links
        .route(
            "/by-channel/{channel_id}/invites",
            get(handlers::voice_channels::list_invite_links)
                .post(handlers::voice_channels::create_invite_link),
        )
        .route("/by-channel/{channel_id}/invites/{link_id}", delete(handlers::voice_channels::revoke_invite_link))
        .route("/invites/{code}/use", post(handlers::voice_channels::use_invite_link))
        // Themes
        .route("/themes/{guild_id}", get(handlers::voice_channels::list_themes).post(handlers::voice_channels::create_theme))
        .route("/themes/{guild_id}/{theme_id}", patch(handlers::voice_channels::update_theme).delete(handlers::voice_channels::delete_theme))
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/voice-channels", voice_inner())
}

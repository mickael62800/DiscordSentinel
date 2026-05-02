//! Routes progression: conduct, levels, role panels, auto-roles.

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn conduct_inner() -> Router<AppState> {
    Router::new()
        .route("/config/{guild_id}", get(handlers::community::conduct::get_config))
        .route("/config", post(handlers::community::conduct::save_config))
        .route("/{guild_id}/{user_id}", get(handlers::community::conduct::get_points))
        .route("/{guild_id}/leaderboard", get(handlers::community::conduct::get_leaderboard))
        .route("/{guild_id}/{user_id}/log", get(handlers::community::conduct::get_points_log))
        .route("/{guild_id}/{user_id}/add", post(handlers::community::conduct::add_points))
        // Endpoints appeles par moderation-worker : regen periodique +
        // creation des propositions de ban pour les users a 0 points
        // (cf. WORKERS_ARCHITECTURE_STATE.md P0 #1 + #2).
        .route("/regen-tick", post(handlers::community::conduct::run_regen_tick))
        .route("/sync-ban-proposals", post(handlers::community::conduct::sync_ban_proposals))
}

fn level_inner() -> Router<AppState> {
    Router::new()
        .route("/config/{guild_id}", get(handlers::community::levels::get_config))
        .route("/config", post(handlers::community::levels::save_config))
        .route("/xp", post(handlers::community::levels::add_xp))
        .route("/{guild_id}/{user_id}", get(handlers::community::levels::get_user_level))
        .route("/{guild_id}/leaderboard", get(handlers::community::levels::get_leaderboard))
        .route("/rewards/{guild_id}", get(handlers::community::levels::get_rewards))
        .route("/rewards", post(handlers::community::levels::set_reward))
        .route("/rewards/{guild_id}/{level}", delete(handlers::community::levels::delete_reward))
        // Admin overrides : set valeur exacte XP texte/voix, reset 0.
        .route("/admin/set-xp", post(handlers::community::levels::set_user_xp))
        .route("/admin/reset-xp", post(handlers::community::levels::reset_user_xp))
}

fn role_panel_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::community::role_panels::create_panel))
        .route("/{guild_id}", get(handlers::community::role_panels::list_panels))
        .route("/detail/{panel_id}", get(handlers::community::role_panels::get_panel).delete(handlers::community::role_panels::delete_panel))
        .route("/by-message/{message_id}", get(handlers::community::role_panels::get_panel_by_message))
        .route("/set-message", patch(handlers::community::role_panels::set_message_id))
}

fn auto_role_inner() -> Router<AppState> {
    Router::new()
        .route("/{guild_id}", get(handlers::community::role_panels::list_auto_roles))
        .route("/", post(handlers::community::role_panels::add_auto_role))
        .route("/{guild_id}/{role_id}", delete(handlers::community::role_panels::delete_auto_role))
}

fn announcement_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::community::announcements::create_announcement))
        .route("/{guild_id}", get(handlers::community::announcements::list_announcements))
        .route(
            "/by-id/{id}",
            get(handlers::community::announcements::get_announcement)
                .patch(handlers::community::announcements::update_announcement)
                .delete(handlers::community::announcements::delete_announcement),
        )
        .route("/{id}/toggle", post(handlers::community::announcements::toggle_announcement))
        .route("/{id}/preview", get(handlers::community::announcements::preview_announcement))
        .route("/{id}/runs", get(handlers::community::announcements::list_runs))
        .route(
            "/{id}/interactions",
            get(handlers::community::announcements::list_button_interactions),
        )
        // Worker / bot (interne)
        .route("/internal/due", get(handlers::community::announcements::fetch_due))
        .route(
            "/internal/runs/{run_id}/result",
            post(handlers::community::announcements::record_run_result),
        )
        .route(
            "/internal/button-click",
            post(handlers::community::announcements::record_button_click),
        )
}

fn confession_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::community::confessions::create_confession))
        .route("/{guild_id}/list", get(handlers::community::confessions::list_confessions))
        .route(
            "/by-id/{id}",
            get(handlers::community::confessions::get_confession)
                .patch(handlers::community::confessions::edit_confession)
                .delete(handlers::community::confessions::delete_confession),
        )
        .route(
            "/by-id/{id}/message-refs",
            post(handlers::community::confessions::update_message_refs),
        )
        .route(
            "/by-message-id/{message_id}",
            get(handlers::community::confessions::get_by_message_id),
        )
        .route(
            "/by-id/{confession_id}/replies",
            get(handlers::community::confessions::list_replies)
                .post(handlers::community::confessions::create_reply),
        )
        .route(
            "/replies/{id}/message-id",
            post(handlers::community::confessions::update_reply_message_id),
        )
        .route(
            "/replies/{id}",
            delete(handlers::community::confessions::delete_reply),
        )
        .route("/reports", post(handlers::community::confessions::create_report))
        .route(
            "/{guild_id}/reports",
            get(handlers::community::confessions::list_reports),
        )
        .route(
            "/reports/{id}/resolve",
            post(handlers::community::confessions::resolve_report),
        )
        .route(
            "/config/{guild_id}",
            get(handlers::community::confessions::get_config),
        )
        .route("/config", post(handlers::community::confessions::save_config))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/api/conduct", conduct_inner())
        .nest("/api/levels", level_inner())
        .nest("/api/announcements", announcement_inner())
        .nest("/api/confessions", confession_inner())
        .nest("/api/role-panels", role_panel_inner())
        .nest("/api/auto-roles", auto_role_inner())
}

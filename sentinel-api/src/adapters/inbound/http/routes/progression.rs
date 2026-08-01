//! Routes progression: levels, role panels, auto-roles.

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::routing::put;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn level_inner() -> Router<AppState> {
    Router::new()
        .route("/xp", post(handlers::community::levels::add_xp))
        .route(
            "/{guild_id}/{user_id}",
            get(handlers::community::levels::get_user_level),
        )
        .route(
            "/{guild_id}/leaderboard",
            get(handlers::community::levels::get_leaderboard),
        )
        // Admin overrides : set valeur exacte XP texte/voix, reset 0.
        .route(
            "/admin/set-xp",
            post(handlers::community::levels::set_user_xp),
        )
        .route(
            "/admin/reset-xp",
            post(handlers::community::levels::reset_user_xp),
        )
}

fn role_panel_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::community::role_panels::create_panel))
        .route(
            "/{guild_id}",
            get(handlers::community::role_panels::list_panels),
        )
        .route(
            "/detail/{panel_id}",
            get(handlers::community::role_panels::get_panel)
                .delete(handlers::community::role_panels::delete_panel),
        )
        .route(
            "/by-message/{message_id}",
            get(handlers::community::role_panels::get_panel_by_message),
        )
        .route(
            "/set-message",
            patch(handlers::community::role_panels::set_message_id),
        )
}

fn auto_role_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}",
            get(handlers::community::role_panels::list_auto_roles),
        )
        .route("/", post(handlers::community::role_panels::add_auto_role))
        .route(
            "/{guild_id}/{role_id}",
            delete(handlers::community::role_panels::delete_auto_role),
        )
}

fn announcement_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            post(handlers::community::announcements::create_announcement),
        )
        // ── Planning communautaire ──
        // Les evenements sont sous /api/events/{guild_id} et le detail sous
        // /api/events/detail/{id} : un segment fixe evite l'ambiguite entre un
        // guild_id (snowflake) et un uuid d'evenement.
        .route(
            "/api/events/{guild_id}",
            get(handlers::community::events::list_events)
                .post(handlers::community::events::create_event),
        )
        .route(
            "/api/events/detail/{id}",
            get(handlers::community::events::get_event)
                .put(handlers::community::events::update_event)
                .delete(handlers::community::events::delete_event),
        )
        .route(
            "/api/events/detail/{id}/join",
            post(handlers::community::events::join_event)
                .delete(handlers::community::events::leave_event),
        )
        // ── Vie de la communaute ──
        // Meme convention que le planning : `{guild_id}` pour la collection,
        // `detail/{id}` pour l'element, un segment fixe evitant l'ambiguite
        // entre un snowflake et un uuid.
        .route(
            "/api/lfg/{guild_id}",
            get(handlers::community::lfg::list_lfg)
                .post(handlers::community::lfg::create_lfg),
        )
        .route(
            "/api/lfg/detail/{id}",
            delete(handlers::community::lfg::delete_lfg),
        )
        .route(
            "/api/lfg/detail/{id}/close",
            post(handlers::community::lfg::close_lfg),
        )
        .route(
            "/api/lfg/detail/{id}/join",
            post(handlers::community::lfg::join_lfg)
                .delete(handlers::community::lfg::leave_lfg),
        )
        .route(
            "/api/polls/{guild_id}",
            get(handlers::community::polls::list_polls)
                .post(handlers::community::polls::create_poll),
        )
        .route(
            "/api/polls/detail/{id}",
            delete(handlers::community::polls::delete_poll),
        )
        .route(
            "/api/polls/detail/{id}/close",
            post(handlers::community::polls::close_poll),
        )
        .route(
            "/api/polls/detail/{id}/vote",
            post(handlers::community::polls::vote_poll),
        )
        // La suppression d'une designation porte le guild_id dans l'URL : le
        // controle RBAC est par guilde, il lui faut cette information avant
        // d'aller chercher la ligne.
        .route(
            "/api/spotlight/{guild_id}",
            get(handlers::community::spotlight::list_spotlight)
                .post(handlers::community::spotlight::designate_spotlight),
        )
        .route(
            "/api/spotlight/{guild_id}/detail/{id}",
            delete(handlers::community::spotlight::delete_spotlight),
        )
        .route(
            "/api/news/{guild_id}",
            get(handlers::community::news::list_news)
                .post(handlers::community::news::create_news),
        )
        .route(
            "/api/news/detail/{id}",
            put(handlers::community::news::update_news)
                .delete(handlers::community::news::delete_news),
        )
        .route(
            "/{guild_id}",
            get(handlers::community::announcements::list_announcements),
        )
        .route(
            "/by-id/{id}",
            get(handlers::community::announcements::get_announcement)
                .patch(handlers::community::announcements::update_announcement)
                .delete(handlers::community::announcements::delete_announcement),
        )
        .route(
            "/{id}/toggle",
            post(handlers::community::announcements::toggle_announcement),
        )
        .route(
            "/{id}/preview",
            get(handlers::community::announcements::preview_announcement),
        )
        .route(
            "/{id}/runs",
            get(handlers::community::announcements::list_runs),
        )
        .route(
            "/{id}/interactions",
            get(handlers::community::announcements::list_button_interactions),
        )
        // Worker / bot (interne)
        .route(
            "/internal/due",
            get(handlers::community::announcements::fetch_due),
        )
        .route(
            "/internal/runs/{run_id}/result",
            post(handlers::community::announcements::record_run_result),
        )
        .route(
            "/internal/button-click",
            post(handlers::community::announcements::record_button_click),
        )
        .route(
            "/internal/retention-cleanup",
            post(handlers::community::announcements::retention_cleanup_all),
        )
}

fn confession_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            post(handlers::community::confessions::create_confession),
        )
        .route(
            "/{guild_id}/list",
            get(handlers::community::confessions::list_confessions),
        )
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
        .route(
            "/reports",
            post(handlers::community::confessions::create_report),
        )
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
        .route(
            "/config",
            post(handlers::community::confessions::save_config),
        )
}

fn age_ban_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::community::age_bans::create_age_ban))
        .route(
            "/due",
            get(handlers::community::age_bans::list_due_age_bans),
        )
        .route(
            "/{id}/lift",
            post(handlers::community::age_bans::lift_age_ban),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/api/levels", level_inner())
        .nest("/api/announcements", announcement_inner())
        .nest("/api/confessions", confession_inner())
        .nest("/api/role-panels", role_panel_inner())
        .nest("/api/auto-roles", auto_role_inner())
        .nest("/api/age-bans", age_ban_inner())
}

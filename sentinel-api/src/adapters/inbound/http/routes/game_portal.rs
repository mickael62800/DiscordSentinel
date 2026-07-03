//! Routes Game Portal — toutes les actions sensibles passent par
//! component_gates (configurable depuis la page RBAC).

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Templates (catalogue)
        .route(
            "/api/games/{guild_id}/templates",
            get(handlers::game::templates::list_templates_for_guild),
        )
        .route(
            "/api/games/templates/{id}",
            get(handlers::game::templates::get_template),
        )
        // Serveurs CRUD + lifecycle
        .route(
            "/api/games/{guild_id}/servers",
            get(handlers::game::servers::list_servers).post(handlers::game::servers::create_server),
        )
        .route(
            "/api/games/servers/{server_id}",
            get(handlers::game::servers::get_server).delete(handlers::game::servers::delete_server),
        )
        .route(
            "/api/games/servers/{server_id}/start",
            post(handlers::game::servers::start_server),
        )
        .route(
            "/api/games/servers/{server_id}/stop",
            post(handlers::game::servers::stop_server),
        )
        .route(
            "/api/games/servers/{server_id}/restart",
            post(handlers::game::servers::restart_server),
        )
        // Observabilite
        .route(
            "/api/games/servers/{server_id}/logs",
            get(handlers::game::servers::get_logs),
        )
        .route(
            "/api/games/servers/{server_id}/stats",
            get(handlers::game::servers::get_stats),
        )
        // Config + console RCON
        .route(
            "/api/games/servers/{server_id}/config",
            put(handlers::game::servers::update_config),
        )
        .route(
            "/api/games/servers/{server_id}/command",
            post(handlers::game::servers::execute_rcon),
        )
        .route(
            "/api/games/servers/{server_id}/sessions",
            get(handlers::game::sessions::list_sessions),
        )
        // Evenements de serveur : role par template, inscriptions, salons.
        .route(
            "/api/games/{guild_id}/template-settings",
            get(handlers::game::session_events::list_template_settings),
        )
        .route(
            "/api/games/{guild_id}/template-settings/{slug}",
            put(handlers::game::session_events::set_template_role),
        )
        .route(
            "/api/games/servers/{server_id}/registrations",
            get(handlers::game::session_events::list_registrations)
                .post(handlers::game::session_events::register_player),
        )
        .route(
            "/api/games/servers/{server_id}/registrations/{user_id}",
            delete(handlers::game::session_events::unregister_player),
        )
        .route(
            "/api/games/servers/{server_id}/session-channels",
            patch(handlers::game::session_events::set_session_channels),
        )
        // Endpoints internes pour game-portal-worker (auth via API key,
        // pas de RBAC user — le worker est de confiance).
        .route(
            "/api/games/internal/jobs/health-check",
            post(handlers::game::jobs::job_health_check),
        )
        .route(
            "/api/games/internal/jobs/idle-shutdown",
            post(handlers::game::jobs::job_idle_shutdown),
        )
        .route(
            "/api/games/internal/jobs/reconcile",
            post(handlers::game::jobs::job_reconcile),
        )
        .route(
            "/api/games/internal/jobs/image-cleanup",
            post(handlers::game::jobs::job_image_cleanup),
        )
}

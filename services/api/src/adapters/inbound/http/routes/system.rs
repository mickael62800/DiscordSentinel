//! Routes systeme (user activity, models status, cache, system info, welcome, jobs async, RBAC).

use axum::routing::delete;
use axum::routing::get;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // User activity (surveillance)
        .route("/api/user-activity", post(handlers::audit::user_activity::create_activity))
        .route("/api/user-activity/{guild_id}/{user_id}", get(handlers::audit::user_activity::get_activity))
        // Models status (IA)
        .route("/api/models/status", get(handlers::system::models_status::get_models_status))
        .route("/api/models/reload", post(handlers::system::models_status::reload_model))
        // Cache monitoring
        .route("/api/cache/stats", get(handlers::system::cache_stats::get_cache_stats))
        // Detail systeme (bots/workers list + CPU/RAM host + uptime + taille BDD)
        .route("/api/system/info", get(handlers::system::info::get_system_info))
        // Welcome config
        .route("/api/welcome/{guild_id}", get(handlers::community::welcome::get_config).put(handlers::community::welcome::save_config))
        // Phase 4 A — File d'attente IA async (POST = enqueue, GET = statut)
        .route("/api/ai/jobs", post(handlers::ai::ai_jobs::create_ai_job))
        .route("/api/ai/jobs/{id}", get(handlers::ai::ai_jobs::get_ai_job))
        // Phase 6 A — File d'attente exports async (infractions/audit_logs/moderation_actions, CSV/JSON)
        .route("/api/exports/jobs", post(handlers::system::exports::create_export_job))
        .route("/api/exports/jobs/{id}", get(handlers::system::exports::get_export_job))
        // Phase 7 B — Endpoints RBAC CRUD (gated via require_role dans les handlers)
        .route("/api/rbac/guilds/{guild_id}/users", get(handlers::system::rbac::list_guild_users))
        .route(
            "/api/rbac/guilds/{guild_id}/users/{user_id}",
            post(handlers::system::rbac::grant_role)
                .patch(handlers::system::rbac::update_role)
                .delete(handlers::system::rbac::revoke_role),
        )
        .route("/api/rbac/me/{guild_id}", get(handlers::system::rbac::get_my_role))
        // Phase 1 sync Discord <-> Web : mapping action_id <-> Discord message
        .route(
            "/api/discord-messages/register",
            post(handlers::audit::discord_action_messages::register),
        )
        .route(
            "/api/discord-messages/{action_id}",
            get(handlers::audit::discord_action_messages::list_for_action),
        )
        .route(
            "/api/discord-messages/{action_id}/{kind}",
            delete(handlers::audit::discord_action_messages::delete_mapping),
        )
}

//! Routes systeme (user activity, models status, cache, system info, welcome, jobs async, RBAC).

use axum::routing::{delete, get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // User activity (surveillance)
        .route("/api/user-activity", post(handlers::user_activity::create_activity))
        .route("/api/user-activity/{guild_id}/{user_id}", get(handlers::user_activity::get_activity))
        // Models status (IA)
        .route("/api/models/status", get(handlers::models_status::get_models_status))
        .route("/api/models/reload", post(handlers::models_status::reload_model))
        // Cache monitoring
        .route("/api/cache/stats", get(handlers::cache_stats::get_cache_stats))
        // Detail systeme (bots/workers list + CPU/RAM host + uptime + taille BDD)
        .route("/api/system/info", get(handlers::system::get_system_info))
        // Welcome config
        .route("/api/welcome/{guild_id}", get(handlers::welcome::get_config).put(handlers::welcome::save_config))
        // Phase 4 A — File d'attente IA async (POST = enqueue, GET = statut)
        .route("/api/ai/jobs", post(handlers::ai_jobs::create_ai_job))
        .route("/api/ai/jobs/{id}", get(handlers::ai_jobs::get_ai_job))
        // Phase 6 A — File d'attente exports async (infractions/audit_logs/moderation_actions, CSV/JSON)
        .route("/api/exports/jobs", post(handlers::exports::create_export_job))
        .route("/api/exports/jobs/{id}", get(handlers::exports::get_export_job))
        // Phase 7 B — Endpoints RBAC CRUD (gated via require_role dans les handlers)
        .route("/api/rbac/guilds/{guild_id}/users", get(handlers::rbac::list_guild_users))
        .route(
            "/api/rbac/guilds/{guild_id}/users/{user_id}",
            post(handlers::rbac::grant_role)
                .patch(handlers::rbac::update_role)
                .delete(handlers::rbac::revoke_role),
        )
        .route("/api/rbac/me/{guild_id}", get(handlers::rbac::get_my_role))
        // Phase 1 sync Discord <-> Web : mapping action_id <-> Discord message
        .route(
            "/api/discord-messages/register",
            post(handlers::discord_action_messages::register),
        )
        .route(
            "/api/discord-messages/{action_id}",
            get(handlers::discord_action_messages::list_for_action),
        )
        .route(
            "/api/discord-messages/{action_id}/{kind}",
            delete(handlers::discord_action_messages::delete_mapping),
        )
}

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
        .route("/api/user-activity/{guild_id}/by-message/{message_id}", get(handlers::audit::user_activity::get_by_message_id))
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
        // Phase Docker — administration via /var/run/docker.sock (gate superadmin sur les actions)
        .route("/api/docker/overview", get(handlers::system::docker::get_overview))
        .route("/api/docker/containers", get(handlers::system::docker::list_containers))
        .route("/api/docker/containers/{id}", delete(handlers::system::docker::remove_container))
        .route("/api/docker/containers/{id}/start", post(handlers::system::docker::start_container))
        .route("/api/docker/containers/{id}/stop", post(handlers::system::docker::stop_container))
        .route("/api/docker/containers/{id}/restart", post(handlers::system::docker::restart_container))
        .route("/api/docker/containers/{id}/logs", get(handlers::system::docker::container_logs))
        .route("/api/docker/images", get(handlers::system::docker::list_images))
        .route("/api/docker/images/{id}", delete(handlers::system::docker::remove_image))
        .route("/api/docker/volumes", get(handlers::system::docker::list_volumes))
        .route("/api/docker/volumes/{name}", delete(handlers::system::docker::remove_volume))
        .route("/api/docker/networks", get(handlers::system::docker::list_networks))
        .route("/api/docker/prune/containers", post(handlers::system::docker::prune_containers))
        .route("/api/docker/prune/images", post(handlers::system::docker::prune_images))
        .route("/api/docker/prune/volumes", post(handlers::system::docker::prune_volumes))
        .route("/api/docker/prune/networks", post(handlers::system::docker::prune_networks))
        .route("/api/docker/prune/system", post(handlers::system::docker::prune_system))
        // AI dataset (collecte messages -> CSV pour entrainement)
        .route(
            "/api/ai-dataset/messages/{guild_id}",
            get(handlers::ai::dataset::list_messages)
                .delete(handlers::ai::dataset::bulk_delete),
        )
        // Invitations a usage unique (owner+ pour gerer, auth pour redeem)
        .route("/api/invitations", post(handlers::system::invitations::create_invitation))
        .route(
            "/api/invitations/{guild_id}",
            get(handlers::system::invitations::list_invitations),
        )
        .route(
            "/api/invitations/code/{code}",
            delete(handlers::system::invitations::revoke_invitation),
        )
        .route(
            "/api/auth/redeem-invitation",
            post(handlers::system::invitations::redeem_invitation),
        )
        .route(
            "/api/auth/check-access",
            get(handlers::system::invitations::check_access),
        )
        // Security monitoring (admin+) : top IPs, auth failures, audit logs, TLS
        .route("/api/security/top-ips", get(handlers::system::security::top_ips))
        .route("/api/security/auth-failures", get(handlers::system::security::auth_failures))
        .route("/api/security/banned-ips", get(handlers::system::security::banned_ips))
        .route("/api/security/audit-logs", get(handlers::system::security::audit_logs))
        .route("/api/security/tls-cert", get(handlers::system::security::tls_cert))
        .route("/api/security/traffic-trend", get(handlers::system::security::traffic_trend))
        .route("/api/security/last-logins", get(handlers::system::security::last_successful_logins))
        .route(
            "/api/security/cleanup",
            delete(handlers::system::security::cleanup_security_logs),
        )
        .route(
            "/api/security/server-events",
            get(handlers::system::server_events::list_server_events),
        )
        .route(
            "/api/security/ban-ip",
            post(handlers::system::security::ban_ip),
        )
        .route(
            "/api/security/unban-ip",
            post(handlers::system::security::unban_ip),
        )
        // RBAC component visibility (overrides UI par role)
        .route(
            "/api/rbac/component-visibility/{guild_id}",
            get(handlers::system::component_visibility::list_visibility)
                .put(handlers::system::component_visibility::upsert_visibility),
        )
}

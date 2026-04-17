//! Routes dashboard & config (guildes, logs, infractions, bots, IA, purge).

use axum::routing::{delete, get, patch, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn dashboard_inner() -> Router<AppState> {
    Router::new()
        .route("/guilds", get(handlers::dashboard::list_guilds))
        .route("/guilds/register", post(handlers::dashboard::register_guild))
        .route("/logs", get(handlers::dashboard::get_logs).post(handlers::dashboard::create_log))
        .route("/logs/{category}", delete(handlers::dashboard::delete_logs_by_category))
        .route("/infractions", get(handlers::dashboard::get_all_infractions))
        .route("/infractions/{id}", delete(handlers::infractions::delete_infraction))
        .route("/rules", get(handlers::dashboard::get_all_rules))
        .route("/rules/{id}", patch(handlers::dashboard::toggle_rule))
        .route("/bots/heartbeat", post(handlers::dashboard::bot_heartbeat))
        .route("/bots/definitions", get(handlers::bot_config::get_definitions))
        .route("/bots/config/{guild_id}", get(handlers::bot_config::get_guild_config))
        .route("/bots/config/{guild_id}/{bot_name}", get(handlers::bot_config::get_bot_config))
        .route("/bots/config", post(handlers::bot_config::set_config).delete(handlers::bot_config::delete_config))
        .route("/ia-config/{guild_id}", get(handlers::ia_config::get_ia_config).put(handlers::ia_config::save_ia_config))
        .route("/purge/infractions", delete(handlers::purge::purge_infractions))
        .route("/purge/audit-logs", delete(handlers::purge::purge_audit_logs))
        .route("/purge/logs", delete(handlers::purge::purge_logs))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // Dashboard & config routes
        .nest("/api", dashboard_inner())
        // Charts
        .route("/api/charts/activity", get(handlers::dashboard_charts::get_activity_trend))
}

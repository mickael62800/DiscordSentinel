mod application;
mod domain;
mod infrastructure;
mod presentation;

use std::sync::Arc;

use application::auth_service::AuthService;
use application::dashboard_service::DashboardService;
use application::infractions_service::InfractionsService;
use application::logs_service::LogsService;
use application::moderation_service::ModerationService;
use application::realtime_service::RealtimeService;
use application::rules_service::RulesService;
use application::security_service::SecurityService;
use application::tickets_service::TicketsService;
use application::bot_config_service::BotConfigService;
use application::guild_service::GuildService;
use application::voice_channels_service::VoiceChannelsService;
use application::conduct_service::ConductService;
use application::audit_logs_service::AuditLogsService;
use application::dashboard_charts_service::DashboardChartsService;
use application::levels_service::LevelsService;
use application::role_panels_service::RolePanelsService;
use application::discord_roles_service::DiscordRolesService;
use application::watched_users_service::WatchedUsersService;
use application::members_service::MembersService;
use application::coude_service::CoudeService;
use domain::ports::AppAdapter;
use infrastructure::api_adapter::ApiAdapter;
use infrastructure::config_store::ConfigStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_store = ConfigStore::new().expect("Failed to initialize LMDB config store");
    let api_config = config_store.get_api_config().ok().flatten();

    let api_adapter = match api_config {
        Some(ref cfg) => {
            println!("Using API adapter: {}", cfg.api_url);
            Arc::new(ApiAdapter::new(cfg.api_url.clone(), cfg.api_key.clone()))
        }
        None => {
            let default_url = std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
            let default_key = std::env::var("API_KEY").unwrap_or_default();
            println!("No API config found, using API adapter with default: {}", default_url);
            Arc::new(ApiAdapter::new(default_url, default_key))
        }
    };
    let adapter: Arc<dyn AppAdapter> = api_adapter.clone();

    let auth_svc = Arc::new(AuthService::new(config_store));
    let realtime_svc = Arc::new(RealtimeService::new());
    let dashboard_svc = Arc::new(DashboardService::new(adapter.clone()));
    let logs_svc = Arc::new(LogsService::new(adapter.clone()));
    let infractions_svc = Arc::new(InfractionsService::new(adapter.clone()));
    let rules_svc = Arc::new(RulesService::new(adapter.clone()));
    let tickets_svc = Arc::new(TicketsService::new(adapter.clone()));
    let security_svc = Arc::new(SecurityService::new(adapter.clone()));
    let moderation_svc = Arc::new(ModerationService::new(adapter.clone()));
    let bot_config_svc = Arc::new(BotConfigService::new(adapter.clone()));
    let guild_svc = Arc::new(GuildService::new(adapter.clone()));
    let voice_channels_svc = Arc::new(VoiceChannelsService::new(adapter.clone()));
    let conduct_svc = Arc::new(ConductService::new(adapter.clone()));
    let audit_logs_svc = Arc::new(AuditLogsService::new(adapter.clone()));
    let dashboard_charts_svc = Arc::new(DashboardChartsService::new(adapter.clone()));
    let levels_svc = Arc::new(LevelsService::new(adapter.clone()));
    let role_panels_svc = Arc::new(RolePanelsService::new(adapter.clone()));
    let discord_roles_svc = Arc::new(DiscordRolesService::new(adapter.clone()));
    let watched_users_svc = Arc::new(WatchedUsersService::new(adapter.clone()));
    let members_svc = Arc::new(MembersService::new(adapter.clone()));
    let coude_svc = Arc::new(CoudeService::new(adapter.clone()));

    // IA config uses direct HTTP (no repository trait needed)
    let (ia_base_url, ia_api_key) = match &api_config {
        Some(cfg) => (cfg.api_url.clone(), cfg.api_key.clone()),
        None => (
            std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
            std::env::var("API_KEY").unwrap_or_default(),
        ),
    };
    let ia_config_svc = Arc::new(application::ia_config_service::IaConfigService::new(ia_base_url.clone(), ia_api_key.clone()));
    let analytics_svc = Arc::new(application::analytics_service::AnalyticsService::new(ia_base_url, ia_api_key));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(api_adapter)
        .manage(auth_svc)
        .manage(realtime_svc)
        .manage(dashboard_svc)
        .manage(logs_svc)
        .manage(infractions_svc)
        .manage(rules_svc)
        .manage(tickets_svc)
        .manage(security_svc)
        .manage(moderation_svc)
        .manage(bot_config_svc)
        .manage(guild_svc)
        .manage(voice_channels_svc)
        .manage(conduct_svc)
        .manage(audit_logs_svc)
        .manage(dashboard_charts_svc)
        .manage(levels_svc)
        .manage(role_panels_svc)
        .manage(discord_roles_svc)
        .manage(watched_users_svc)
        .manage(members_svc)
        .manage(coude_svc)
        .manage(ia_config_svc)
        .manage(analytics_svc)
        .invoke_handler(tauri::generate_handler![
            presentation::commands::discord_login,
            presentation::commands::get_current_user,
            presentation::commands::logout,
            presentation::commands::has_discord_config,
            presentation::commands::get_discord_config,
            presentation::commands::save_discord_config,
            presentation::commands::clear_discord_config,
            presentation::commands::get_api_config,
            presentation::commands::save_api_config,
            presentation::commands::ws_connect,
            presentation::commands::ws_disconnect,
            presentation::commands::ws_status,
            presentation::commands::get_guilds,
            presentation::commands::get_guild_members,
            presentation::commands::get_dashboard_stats,
            presentation::commands::get_logs,
            presentation::commands::delete_logs_by_category,
            presentation::commands::get_infractions,
            presentation::commands::delete_infraction,
            presentation::commands::get_rules,
            presentation::commands::toggle_rule,
            presentation::commands::update_rule,
            presentation::commands::get_tickets,
            presentation::commands::get_ticket_detail,
            presentation::commands::reply_ticket,
            presentation::commands::close_ticket,
            presentation::commands::assign_ticket,
            presentation::commands::get_security_events,
            presentation::commands::log_moderation_action,
            presentation::commands::get_moderation_history,
            presentation::commands::get_confirmed_bans,
            presentation::commands::execute_ban,
            presentation::commands::execute_unban,
            presentation::commands::get_voice_channels,
            presentation::commands::get_voice_channel_detail,
            presentation::commands::get_bot_definitions,
            presentation::commands::get_bot_guild_config,
            presentation::commands::set_bot_config,
            presentation::commands::delete_bot_config,
            presentation::commands::get_conduct_config,
            presentation::commands::get_conduct_leaderboard,
            presentation::commands::get_conduct_points,
            presentation::commands::get_conduct_log,
            presentation::commands::get_level_config,
            presentation::commands::get_level_leaderboard,
            presentation::commands::get_level_rewards,
            presentation::commands::get_role_panels,
            presentation::commands::get_role_panel_detail,
            presentation::commands::get_auto_roles,
            presentation::commands::get_discord_roles,
            presentation::commands::get_activity_trend,
            presentation::commands::get_top_users,
            presentation::commands::get_audit_logs,
            presentation::commands::get_watched_users,
            presentation::commands::get_user_dossier,
            presentation::commands::remove_watched_user,
            presentation::commands::get_ia_config,
            presentation::commands::save_ia_config,
            presentation::commands::get_full_analytics,
            presentation::commands::ai_get_datasets,
            presentation::commands::ai_upload_dataset,
            presentation::commands::ai_start_training,
            presentation::commands::ai_training_status,
            presentation::commands::ai_stop_training,
            presentation::commands::adjust_conduct_points,
            presentation::commands::ai_export_onnx,
            presentation::commands::get_members,
            presentation::commands::get_member,
            presentation::commands::get_member_summary,
            presentation::commands::get_coude_combats,
            presentation::commands::get_coude_players,
            presentation::commands::cancel_coude_combat,
            presentation::commands::adjust_coude_coins,
            presentation::commands::save_bot_token,
            presentation::commands::get_bot_token,
            presentation::commands::get_all_bot_tokens,
            presentation::commands::delete_bot_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

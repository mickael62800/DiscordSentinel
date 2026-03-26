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
use domain::ports::AppAdapter;
use infrastructure::api_adapter::ApiAdapter;
use infrastructure::config_store::ConfigStore;
use infrastructure::mock_adapter::MockAdapter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_store = ConfigStore::new().expect("Failed to initialize LMDB config store");
    let api_config = config_store.get_api_config().ok().flatten();

    let adapter: Arc<dyn AppAdapter> = match api_config {
        Some(ref cfg) => {
            println!("Using API adapter: {}", cfg.api_url);
            Arc::new(ApiAdapter::new(cfg.api_url.clone(), cfg.api_key.clone()))
        }
        None => {
            println!("No API config found, using mock adapter");
            Arc::new(MockAdapter::new())
        }
    };

    let auth_svc = Arc::new(AuthService::new(config_store));
    let realtime_svc = Arc::new(RealtimeService::new());
    let dashboard_svc = Arc::new(DashboardService::new(adapter.clone()));
    let logs_svc = Arc::new(LogsService::new(adapter.clone()));
    let infractions_svc = Arc::new(InfractionsService::new(adapter.clone()));
    let rules_svc = Arc::new(RulesService::new(adapter.clone()));
    let tickets_svc = Arc::new(TicketsService::new(adapter.clone()));
    let security_svc = Arc::new(SecurityService::new(adapter.clone()));
    let moderation_svc = Arc::new(ModerationService::new(adapter.clone()));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .manage(auth_svc)
        .manage(realtime_svc)
        .manage(dashboard_svc)
        .manage(logs_svc)
        .manage(infractions_svc)
        .manage(rules_svc)
        .manage(tickets_svc)
        .manage(security_svc)
        .manage(moderation_svc)
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
            presentation::commands::get_dashboard_stats,
            presentation::commands::get_logs,
            presentation::commands::get_infractions,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

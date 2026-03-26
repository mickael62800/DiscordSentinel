use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::application::auth_service::AuthService;
use crate::application::dashboard_service::DashboardService;
use crate::application::infractions_service::InfractionsService;
use crate::application::logs_service::LogsService;
use crate::application::moderation_service::ModerationService;
use crate::application::realtime_service::{RealtimeService, WsStatus};
use crate::application::rules_service::RulesService;
use crate::application::security_service::SecurityService;
use crate::application::tickets_service::TicketsService;
use crate::domain::entities::{ApiConfig, DiscordConfig, DiscordUser, Infraction, LogEntry, ModerationActionRequest, ModerationActionResponse, ModerationRule, SecurityEvent, ServerStats, Ticket, TicketDetail, UpdateRuleParams, UserModerationHistory};

#[tauri::command]
pub async fn get_dashboard_stats(
    service: State<'_, Arc<DashboardService>>,
) -> Result<ServerStats, String> {
    service.get_stats().await
}

#[tauri::command]
pub async fn get_logs(service: State<'_, Arc<LogsService>>) -> Result<Vec<LogEntry>, String> {
    service.get_logs().await
}

#[tauri::command]
pub async fn get_infractions(
    service: State<'_, Arc<InfractionsService>>,
) -> Result<Vec<Infraction>, String> {
    service.get_infractions().await
}

#[tauri::command]
pub async fn get_rules(
    service: State<'_, Arc<RulesService>>,
) -> Result<Vec<ModerationRule>, String> {
    service.get_rules().await
}

#[tauri::command]
pub async fn toggle_rule(
    service: State<'_, Arc<RulesService>>,
    id: String,
    enabled: bool,
) -> Result<bool, String> {
    service.toggle_rule(id, enabled).await
}

#[tauri::command]
pub async fn update_rule(
    service: State<'_, Arc<RulesService>>,
    guild_id: String,
    flag_type: String,
    weight: f64,
    threshold_warn: f64,
    threshold_delete: f64,
    threshold_mute: f64,
    threshold_ban: f64,
    enabled: bool,
) -> Result<(), String> {
    service.update_rule(UpdateRuleParams {
        guild_id,
        flag_type,
        weight,
        threshold_warn,
        threshold_delete,
        threshold_mute,
        threshold_ban,
        enabled,
    }).await
}

#[tauri::command]
pub async fn get_tickets(
    service: State<'_, Arc<TicketsService>>,
) -> Result<Vec<Ticket>, String> {
    service.get_tickets().await
}

#[tauri::command]
pub async fn get_ticket_detail(
    service: State<'_, Arc<TicketsService>>,
    id: String,
) -> Result<TicketDetail, String> {
    service.get_ticket_detail(id).await
}

#[tauri::command]
pub async fn reply_ticket(
    service: State<'_, Arc<TicketsService>>,
    ticket_id: String,
    content: String,
) -> Result<(), String> {
    service.reply_ticket(ticket_id, content).await
}

#[tauri::command]
pub async fn close_ticket(
    service: State<'_, Arc<TicketsService>>,
    id: String,
) -> Result<(), String> {
    service.close_ticket(id).await
}

#[tauri::command]
pub async fn assign_ticket(
    service: State<'_, Arc<TicketsService>>,
    id: String,
    assignee: String,
) -> Result<(), String> {
    service.assign_ticket(id, assignee).await
}

// --- Auth commands ---

#[tauri::command]
pub async fn discord_login(
    service: State<'_, Arc<AuthService>>,
) -> Result<DiscordUser, String> {
    service.start_oauth_flow().await
}

#[tauri::command]
pub fn get_current_user(
    service: State<'_, Arc<AuthService>>,
) -> Option<DiscordUser> {
    service.get_current_user()
}

#[tauri::command]
pub fn logout(service: State<'_, Arc<AuthService>>) {
    service.logout();
}

// --- Config commands ---

#[tauri::command]
pub fn has_discord_config(
    service: State<'_, Arc<AuthService>>,
) -> Result<bool, String> {
    Ok(service.get_discord_config()?.is_some())
}

#[tauri::command]
pub fn get_discord_config(
    service: State<'_, Arc<AuthService>>,
) -> Result<Option<DiscordConfig>, String> {
    service.get_discord_config()
}

#[tauri::command]
pub fn save_discord_config(
    service: State<'_, Arc<AuthService>>,
    client_id: String,
    client_secret: String,
) -> Result<(), String> {
    service.save_discord_config(DiscordConfig { client_id, client_secret })
}

#[tauri::command]
pub fn clear_discord_config(
    service: State<'_, Arc<AuthService>>,
) -> Result<(), String> {
    service.clear_discord_config()
}

#[tauri::command]
pub fn get_api_config(
    service: State<'_, Arc<AuthService>>,
) -> Result<Option<ApiConfig>, String> {
    service.get_api_config()
}

#[tauri::command]
pub fn save_api_config(
    service: State<'_, Arc<AuthService>>,
    api_url: String,
    api_key: String,
) -> Result<(), String> {
    service.save_api_config(ApiConfig { api_url, api_key })
}

// --- WebSocket commands ---

#[tauri::command]
pub async fn ws_connect(
    app: AppHandle,
    service: State<'_, Arc<RealtimeService>>,
    auth_service: State<'_, Arc<AuthService>>,
) -> Result<(), String> {
    let api_config = auth_service.get_api_config()?
        .ok_or("API not configured")?;
    service.connect(app, api_config.api_url, api_config.api_key).await
}

#[tauri::command]
pub async fn ws_disconnect(
    service: State<'_, Arc<RealtimeService>>,
) -> Result<(), String> {
    service.disconnect().await;
    Ok(())
}

#[tauri::command]
pub async fn ws_status(
    service: State<'_, Arc<RealtimeService>>,
) -> Result<WsStatus, String> {
    Ok(service.get_status().await)
}

// --- Security commands ---

#[tauri::command]
pub async fn get_security_events(
    service: State<'_, Arc<SecurityService>>,
    guild_id: Option<String>,
) -> Result<Vec<SecurityEvent>, String> {
    service.get_events(guild_id).await
}

// --- Moderation commands ---

#[tauri::command]
pub async fn log_moderation_action(
    service: State<'_, Arc<ModerationService>>,
    guild_id: String,
    channel_id: String,
    moderator_id: String,
    moderator_name: String,
    target_id: String,
    target_name: String,
    action_type: String,
    reason: String,
    gravity: Option<String>,
    duration: Option<u64>,
) -> Result<ModerationActionResponse, String> {
    service.log_action(ModerationActionRequest {
        guild_id,
        channel_id,
        moderator_id,
        moderator_name,
        target_id,
        target_name,
        action_type,
        reason,
        gravity,
        duration,
    }).await
}

#[tauri::command]
pub async fn get_moderation_history(
    service: State<'_, Arc<ModerationService>>,
    guild_id: String,
    user_id: String,
) -> Result<UserModerationHistory, String> {
    service.get_history(guild_id, user_id).await
}

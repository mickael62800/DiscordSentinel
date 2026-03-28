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
use crate::application::bot_config_service::BotConfigService;
use crate::application::guild_service::GuildService;
use crate::application::voice_channels_service::VoiceChannelsService;
use crate::application::conduct_service::ConductService;
use crate::application::audit_logs_service::AuditLogsService;
use crate::application::dashboard_charts_service::DashboardChartsService;
use crate::application::levels_service::LevelsService;
use crate::application::role_panels_service::RolePanelsService;
use crate::application::watched_users_service::WatchedUsersService;
use crate::application::ia_config_service::{IaConfigService, IaConfig};
use crate::application::analytics_service::{AnalyticsService, FullAnalytics};
use crate::domain::entities::{ApiConfig, AuditLog, AutoRoleConfig, BotDefinition, ConfirmedBan, DailyActivity, LevelConfig, LevelReward, BotGuildConfig, ConductConfig, ConductPointsLog, DiscordConfig, DiscordUser, Guild, Infraction, LogEntry, ModerationActionRequest, ModerationActionResponse, ModerationRule, RolePanel, RolePanelDetail, SecurityEvent, ServerStats, Ticket, TicketDetail, TopUser, UpdateRuleParams, UserConductPoints, UserDossier, UserLevel, UserModerationHistory, VoiceChannel, VoiceChannelDetail, WatchedUser};

#[tauri::command]
pub async fn get_dashboard_stats(
    service: State<'_, Arc<DashboardService>>,
) -> Result<ServerStats, String> {
    service.get_stats().await
}

#[tauri::command]
pub async fn get_guilds(
    service: State<'_, Arc<GuildService>>,
) -> Result<Vec<Guild>, String> {
    service.get_guilds().await
}

#[tauri::command]
pub async fn get_logs(
    service: State<'_, Arc<LogsService>>,
    guild_id: Option<String>,
) -> Result<Vec<LogEntry>, String> {
    service.get_logs(guild_id).await
}

#[tauri::command]
pub async fn delete_logs_by_category(
    service: State<'_, Arc<LogsService>>,
    category: String,
) -> Result<(), String> {
    service.delete_logs_by_category(category).await
}

#[tauri::command]
pub async fn get_infractions(
    service: State<'_, Arc<InfractionsService>>,
    guild_id: Option<String>,
) -> Result<Vec<Infraction>, String> {
    service.get_infractions(guild_id).await
}

#[tauri::command]
pub async fn get_rules(
    service: State<'_, Arc<RulesService>>,
    guild_id: Option<String>,
) -> Result<Vec<ModerationRule>, String> {
    service.get_rules(guild_id).await
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

#[tauri::command]
pub async fn execute_ban(
    service: State<'_, Arc<ModerationService>>,
    guild_id: String,
    user_id: String,
    reason: String,
) -> Result<(), String> {
    service.execute_ban(guild_id, user_id, reason).await
}

#[tauri::command]
pub async fn execute_unban(
    service: State<'_, Arc<ModerationService>>,
    guild_id: String,
    user_id: String,
) -> Result<(), String> {
    service.execute_unban(guild_id, user_id).await
}

#[tauri::command]
pub async fn get_confirmed_bans(
    service: State<'_, Arc<ModerationService>>,
    guild_id: Option<String>,
) -> Result<Vec<ConfirmedBan>, String> {
    service.get_confirmed_bans(guild_id).await
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

// --- Voice Channels commands ---

#[tauri::command]
pub async fn get_voice_channels(
    service: State<'_, Arc<VoiceChannelsService>>,
    guild_id: String,
) -> Result<Vec<VoiceChannel>, String> {
    service.get_channels(guild_id).await
}

#[tauri::command]
pub async fn get_voice_channel_detail(
    service: State<'_, Arc<VoiceChannelsService>>,
    channel_id: String,
) -> Result<VoiceChannelDetail, String> {
    service.get_channel_detail(channel_id).await
}

// --- Bot Config commands ---

#[tauri::command]
pub async fn get_bot_definitions(
    service: State<'_, Arc<BotConfigService>>,
) -> Result<Vec<BotDefinition>, String> {
    service.get_definitions().await
}

#[tauri::command]
pub async fn get_bot_guild_config(
    service: State<'_, Arc<BotConfigService>>,
    guild_id: String,
) -> Result<Vec<BotGuildConfig>, String> {
    service.get_guild_config(guild_id).await
}

#[tauri::command]
pub async fn set_bot_config(
    service: State<'_, Arc<BotConfigService>>,
    guild_id: String,
    bot_name: String,
    config_key: String,
    config_value: String,
) -> Result<(), String> {
    service.set_config(guild_id, bot_name, config_key, config_value).await
}

#[tauri::command]
pub async fn delete_bot_config(
    service: State<'_, Arc<BotConfigService>>,
    guild_id: String,
    bot_name: String,
    config_key: String,
) -> Result<(), String> {
    service.delete_config(guild_id, bot_name, config_key).await
}

// ── Role Panels ──

#[tauri::command]
pub async fn get_role_panels(
    service: State<'_, Arc<RolePanelsService>>,
    guild_id: String,
) -> Result<Vec<RolePanel>, String> {
    service.get_panels(guild_id).await
}

#[tauri::command]
pub async fn get_role_panel_detail(
    service: State<'_, Arc<RolePanelsService>>,
    panel_id: String,
) -> Result<RolePanelDetail, String> {
    service.get_panel(panel_id).await
}

#[tauri::command]
pub async fn get_auto_roles(
    service: State<'_, Arc<RolePanelsService>>,
    guild_id: String,
) -> Result<Vec<AutoRoleConfig>, String> {
    service.get_auto_roles(guild_id).await
}

// ── Dashboard Charts ──

#[tauri::command]
pub async fn get_activity_trend(
    service: State<'_, Arc<DashboardChartsService>>,
    guild_id: Option<String>,
    days: Option<i32>,
) -> Result<Vec<DailyActivity>, String> {
    service.get_activity_trend(guild_id, days).await
}

#[tauri::command]
pub async fn get_top_users(
    service: State<'_, Arc<DashboardChartsService>>,
    guild_id: String,
    limit: Option<u32>,
) -> Result<Vec<TopUser>, String> {
    service.get_top_users(guild_id, limit).await
}

// ── Levels / XP ──

#[tauri::command]
pub async fn get_level_config(
    service: State<'_, Arc<LevelsService>>,
    guild_id: String,
) -> Result<LevelConfig, String> {
    service.get_config(guild_id).await
}

#[tauri::command]
pub async fn get_level_leaderboard(
    service: State<'_, Arc<LevelsService>>,
    guild_id: String,
) -> Result<Vec<UserLevel>, String> {
    service.get_leaderboard(guild_id).await
}

#[tauri::command]
pub async fn get_level_rewards(
    service: State<'_, Arc<LevelsService>>,
    guild_id: String,
) -> Result<Vec<LevelReward>, String> {
    service.get_rewards(guild_id).await
}

// ── Audit Logs ──

#[tauri::command]
pub async fn get_audit_logs(
    service: State<'_, Arc<AuditLogsService>>,
    guild_id: Option<String>,
    event_type: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<AuditLog>, String> {
    service.get_audit_logs(guild_id, event_type, limit).await
}

// ── Watched Users (Surveillance) ──

#[tauri::command]
pub async fn get_watched_users(
    service: State<'_, Arc<WatchedUsersService>>,
    guild_id: Option<String>,
) -> Result<Vec<WatchedUser>, String> {
    service.get_watched_users(guild_id).await
}

#[tauri::command]
pub async fn get_user_dossier(
    service: State<'_, Arc<WatchedUsersService>>,
    guild_id: String,
    user_id: String,
) -> Result<UserDossier, String> {
    service.get_user_dossier(guild_id, user_id).await
}

// ── Conduct ──

#[tauri::command]
pub async fn get_conduct_config(
    service: State<'_, Arc<ConductService>>,
    guild_id: String,
) -> Result<ConductConfig, String> {
    service.get_config(guild_id).await
}

#[tauri::command]
pub async fn get_conduct_leaderboard(
    service: State<'_, Arc<ConductService>>,
    guild_id: String,
) -> Result<Vec<UserConductPoints>, String> {
    service.get_leaderboard(guild_id).await
}

#[tauri::command]
pub async fn get_conduct_points(
    service: State<'_, Arc<ConductService>>,
    guild_id: String,
    user_id: String,
) -> Result<UserConductPoints, String> {
    service.get_points(guild_id, user_id).await
}

#[tauri::command]
pub async fn get_conduct_log(
    service: State<'_, Arc<ConductService>>,
    guild_id: String,
    user_id: String,
) -> Result<Vec<ConductPointsLog>, String> {
    service.get_log(guild_id, user_id).await
}

// ── IA Config ──

#[tauri::command]
pub async fn get_ia_config(
    service: State<'_, Arc<IaConfigService>>,
    guild_id: String,
) -> Result<IaConfig, String> {
    service.get_config(guild_id).await
}

#[tauri::command]
pub async fn save_ia_config(
    service: State<'_, Arc<IaConfigService>>,
    guild_id: String,
    text_enabled: bool,
    text_threshold: f64,
    vision_enabled: bool,
    vision_threshold: f64,
) -> Result<IaConfig, String> {
    service
        .save_config(guild_id, text_enabled, text_threshold, vision_enabled, vision_threshold)
        .await
}

// ── Analytics ──

#[tauri::command]
pub async fn get_full_analytics(
    service: State<'_, Arc<AnalyticsService>>,
    guild_id: Option<String>,
    days: Option<i32>,
) -> Result<FullAnalytics, String> {
    service.get_full_analytics(guild_id, days).await
}

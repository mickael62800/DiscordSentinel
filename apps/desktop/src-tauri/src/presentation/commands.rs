use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::application::auth_service::AuthService;
use crate::infrastructure::api_adapter::ApiAdapter;
use crate::application::coude_service::CoudeService;
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
use crate::application::discord_roles_service::DiscordRolesService;
use crate::application::watched_users_service::WatchedUsersService;
use crate::application::members_service::MembersService;
use crate::application::ia_config_service::{IaConfigService, IaConfig};
use crate::application::analytics_service::{AnalyticsService, FullAnalytics};
use crate::domain::entities::{
    ApiConfig, AuditLog, AutoRoleConfig, BotDefinition, ConfirmedBan, DailyActivity,
    LevelConfig, LevelReward, BotGuildConfig, ConductConfig, ConductPointsLog,
    DiscordConfig, DiscordUser, Guild, GuildMember, Infraction, LogEntry, Member, MemberSummary,
    ModerationActionRequest, ModerationActionResponse, ModerationRule, RolePanel, RolePanelDetail,
    SecurityEvent, ServerStats, Ticket, TicketDetail, TopUser, UpdateRuleParams, UserConductPoints,
    UserDossier, UserLevel, UserModerationHistory, VoiceChannel, VoiceChannelDetail,
    WatchedUser, DiscordRole, CoudeCombat, CoudePlayer,
};

// ─────────────────────────────────────────────────────────────
// Pass-through commands (generated via macro)
// ─────────────────────────────────────────────────────────────

// Dashboard
tauri_passthrough!(get_dashboard_stats, DashboardService, get_stats -> ServerStats);

// Guilds
tauri_passthrough!(get_guilds, GuildService, get_guilds -> Vec<Guild>);
tauri_passthrough!(get_guild_members, GuildService, get_guild_members -> Vec<GuildMember>, guild_id: String);

// Members
tauri_passthrough!(get_members, MembersService, get_members -> Vec<Member>, guild_id: String);
tauri_passthrough!(get_member, MembersService, get_member -> Member, guild_id: String, user_id: String);
tauri_passthrough!(get_member_summary, MembersService, get_member_summary -> MemberSummary, guild_id: String, user_id: String);

// Logs
tauri_passthrough!(get_logs, LogsService, get_logs -> Vec<LogEntry>, guild_id: Option<String>);
tauri_passthrough!(delete_logs_by_category, LogsService, delete_logs_by_category -> (), category: String);

// Infractions
tauri_passthrough!(get_infractions, InfractionsService, get_infractions -> Vec<Infraction>, guild_id: Option<String>);
tauri_passthrough!(delete_infraction, InfractionsService, delete_infraction -> (), id: String);

// Coup de Coude
tauri_passthrough!(get_coude_combats, CoudeService, get_combats -> Vec<CoudeCombat>, guild_id: String, status: Option<String>);
tauri_passthrough!(get_coude_players, CoudeService, get_players -> Vec<CoudePlayer>, guild_id: String);
tauri_passthrough!(cancel_coude_combat, CoudeService, cancel_combat -> (), combat_id: String);
tauri_passthrough!(adjust_coude_coins, CoudeService, adjust_coins -> (), guild_id: String, user_id: String, amount: i64);

// Rules
tauri_passthrough!(get_rules, RulesService, get_rules -> Vec<ModerationRule>, guild_id: Option<String>);
tauri_passthrough!(toggle_rule, RulesService, toggle_rule -> bool, id: String, enabled: bool);

// Tickets
tauri_passthrough!(get_tickets, TicketsService, get_tickets -> Vec<Ticket>);
tauri_passthrough!(get_ticket_detail, TicketsService, get_ticket_detail -> TicketDetail, id: String);
tauri_passthrough!(reply_ticket, TicketsService, reply_ticket -> (), ticket_id: String, content: String);
tauri_passthrough!(close_ticket, TicketsService, close_ticket -> (), id: String);
tauri_passthrough!(assign_ticket, TicketsService, assign_ticket -> (), id: String, assignee: String);

// Moderation
tauri_passthrough!(execute_ban, ModerationService, execute_ban -> (), guild_id: String, user_id: String, reason: String);
tauri_passthrough!(execute_unban, ModerationService, execute_unban -> (), guild_id: String, user_id: String);
tauri_passthrough!(get_confirmed_bans, ModerationService, get_confirmed_bans -> Vec<ConfirmedBan>, guild_id: Option<String>);
tauri_passthrough!(get_moderation_history, ModerationService, get_history -> UserModerationHistory, guild_id: String, user_id: String);

// Security
tauri_passthrough!(get_security_events, SecurityService, get_events -> Vec<SecurityEvent>, guild_id: Option<String>);

// Voice Channels
tauri_passthrough!(get_voice_channels, VoiceChannelsService, get_channels -> Vec<VoiceChannel>, guild_id: String);
tauri_passthrough!(get_voice_channel_detail, VoiceChannelsService, get_channel_detail -> VoiceChannelDetail, channel_id: String);

// Bot Config
tauri_passthrough!(get_bot_definitions, BotConfigService, get_definitions -> Vec<BotDefinition>);
tauri_passthrough!(get_bot_guild_config, BotConfigService, get_guild_config -> Vec<BotGuildConfig>, guild_id: String);
tauri_passthrough!(set_bot_config, BotConfigService, set_config -> (), guild_id: String, bot_name: String, config_key: String, config_value: String);
tauri_passthrough!(delete_bot_config, BotConfigService, delete_config -> (), guild_id: String, bot_name: String, config_key: String);

// Role Panels
tauri_passthrough!(get_role_panels, RolePanelsService, get_panels -> Vec<RolePanel>, guild_id: String);
tauri_passthrough!(get_role_panel_detail, RolePanelsService, get_panel -> RolePanelDetail, panel_id: String);
tauri_passthrough!(get_auto_roles, RolePanelsService, get_auto_roles -> Vec<AutoRoleConfig>, guild_id: String);

// Discord Roles
tauri_passthrough!(get_discord_roles, DiscordRolesService, get_discord_roles -> Vec<DiscordRole>, guild_id: String);

// Dashboard Charts
tauri_passthrough!(get_activity_trend, DashboardChartsService, get_activity_trend -> Vec<DailyActivity>, guild_id: Option<String>, days: Option<i32>);
tauri_passthrough!(get_top_users, DashboardChartsService, get_top_users -> Vec<TopUser>, guild_id: String, limit: Option<u32>);

// Levels / XP
tauri_passthrough!(get_level_config, LevelsService, get_config -> LevelConfig, guild_id: String);
tauri_passthrough!(get_level_leaderboard, LevelsService, get_leaderboard -> Vec<UserLevel>, guild_id: String);
tauri_passthrough!(get_level_rewards, LevelsService, get_rewards -> Vec<LevelReward>, guild_id: String);
tauri_passthrough!(set_level_reward, LevelsService, set_reward -> LevelReward, guild_id: String, level: i32, role_id: String, source: String);
tauri_passthrough!(delete_level_reward, LevelsService, delete_reward -> (), guild_id: String, level: i32, source: String);

// Audit Logs
tauri_passthrough!(get_audit_logs, AuditLogsService, get_audit_logs -> Vec<AuditLog>, guild_id: Option<String>, event_type: Option<String>, limit: Option<i64>);

// Watched Users
tauri_passthrough!(get_watched_users, WatchedUsersService, get_watched_users -> Vec<WatchedUser>, guild_id: Option<String>);
tauri_passthrough!(get_user_dossier, WatchedUsersService, get_user_dossier -> UserDossier, guild_id: String, user_id: String);
tauri_passthrough!(remove_watched_user, WatchedUsersService, remove_watched_user -> (), guild_id: String, user_id: String);

// Conduct
tauri_passthrough!(get_conduct_config, ConductService, get_config -> ConductConfig, guild_id: String);
tauri_passthrough!(get_conduct_leaderboard, ConductService, get_leaderboard -> Vec<UserConductPoints>, guild_id: String);
tauri_passthrough!(get_conduct_points, ConductService, get_points -> UserConductPoints, guild_id: String, user_id: String);
tauri_passthrough!(get_conduct_log, ConductService, get_log -> Vec<ConductPointsLog>, guild_id: String, user_id: String);
tauri_passthrough!(adjust_conduct_points, ConductService, adjust_points -> UserConductPoints, guild_id: String, user_id: String, amount: i32, reason: String);

// IA Config
tauri_passthrough!(get_ia_config, IaConfigService, get_config -> IaConfig, guild_id: String);
tauri_passthrough!(save_ia_config, IaConfigService, save_config -> IaConfig, guild_id: String, text_enabled: bool, text_threshold: f64, vision_enabled: bool, vision_threshold: f64);

// Analytics
tauri_passthrough!(get_full_analytics, AnalyticsService, get_full_analytics -> FullAnalytics, guild_id: Option<String>, days: Option<i32>);

// ─────────────────────────────────────────────────────────────
// Commands with custom logic (kept manual)
// ─────────────────────────────────────────────────────────────

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
    adapter: State<'_, Arc<ApiAdapter>>,
    api_url: String,
    api_key: String,
) -> Result<(), String> {
    service.save_api_config(ApiConfig { api_url: api_url.clone(), api_key: api_key.clone() })?;
    adapter.update_config(api_url, api_key);
    Ok(())
}

// --- Bot Token commands ---

#[tauri::command]
pub fn save_bot_token(
    service: State<'_, Arc<AuthService>>,
    bot_name: String,
    token: String,
) -> Result<(), String> {
    service.save_bot_token(&bot_name, &token)
}

#[tauri::command]
pub fn get_bot_token(
    service: State<'_, Arc<AuthService>>,
    bot_name: String,
) -> Result<Option<String>, String> {
    service.get_bot_token(&bot_name)
}

#[tauri::command]
pub fn get_all_bot_tokens(
    service: State<'_, Arc<AuthService>>,
) -> Result<Vec<(String, bool)>, String> {
    service.get_all_bot_tokens()
}

#[tauri::command]
pub fn delete_bot_token(
    service: State<'_, Arc<AuthService>>,
    bot_name: String,
) -> Result<(), String> {
    service.delete_bot_token(&bot_name)
}

// --- Rules (complex) ---

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

// --- Moderation (complex) ---

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

// ─────────────────────────────────────────────────────────────
// AI Training (proxy to Python API)
// ─────────────────────────────────────────────────────────────

fn ai_api_url() -> String {
    std::env::var("AI_API_URL").unwrap_or_else(|_| "http://localhost:8000".into())
}

#[tauri::command]
pub async fn ai_get_datasets() -> Result<serde_json::Value, String> {
    let resp = reqwest::Client::new()
        .get(format!("{}/api/ai/datasets", ai_api_url()))
        .send().await.map_err(|e| format!("AI API indisponible: {e}"))?;
    resp.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}"))
}

#[tauri::command]
pub async fn ai_upload_dataset(model_type: String, file_path: String) -> Result<serde_json::Value, String> {
    let file_bytes = std::fs::read(&file_path).map_err(|e| format!("Lecture fichier: {e}"))?;
    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "dataset".into());

    let part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);
    let form = reqwest::multipart::Form::new().part("file", part);

    let resp = reqwest::Client::new()
        .post(format!("{}/api/ai/datasets/{}/upload", ai_api_url(), model_type))
        .multipart(form)
        .send().await.map_err(|e| format!("AI API indisponible: {e}"))?;
    resp.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}"))
}

#[tauri::command]
pub async fn ai_start_training(
    model_type: String,
    epochs: u32,
    batch_size: u32,
    learning_rate: f64,
    validation_split: f64,
) -> Result<serde_json::Value, String> {
    let resp = reqwest::Client::new()
        .post(format!("{}/api/ai/training/start", ai_api_url()))
        .json(&serde_json::json!({
            "model_type": model_type,
            "epochs": epochs,
            "batch_size": batch_size,
            "learning_rate": learning_rate,
            "validation_split": validation_split,
        }))
        .send().await.map_err(|e| format!("AI API indisponible: {e}"))?;
    resp.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}"))
}

#[tauri::command]
pub async fn ai_training_status() -> Result<serde_json::Value, String> {
    let resp = reqwest::Client::new()
        .get(format!("{}/api/ai/training/status", ai_api_url()))
        .send().await.map_err(|e| format!("AI API indisponible: {e}"))?;
    resp.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}"))
}

#[tauri::command]
pub async fn ai_stop_training() -> Result<serde_json::Value, String> {
    let resp = reqwest::Client::new()
        .post(format!("{}/api/ai/training/stop", ai_api_url()))
        .send().await.map_err(|e| format!("AI API indisponible: {e}"))?;
    resp.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}"))
}

#[tauri::command]
pub async fn ai_export_onnx(model_type: String) -> Result<serde_json::Value, String> {
    let resp = reqwest::Client::new()
        .post(format!("{}/api/ai/export/{}", ai_api_url(), model_type))
        .send().await.map_err(|e| format!("AI API indisponible: {e}"))?;
    resp.json::<serde_json::Value>().await.map_err(|e| format!("Parse error: {e}"))
}

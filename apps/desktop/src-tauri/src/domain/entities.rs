use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Guild {
    pub guild_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub member_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GuildMember {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BotDefinition {
    pub bot_name: String,
    pub display_name: String,
    pub description: String,
    pub config_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BotGuildConfig {
    pub guild_id: String,
    pub bot_name: String,
    pub config_key: String,
    pub config_value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscordConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiConfig {
    pub api_url: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub global_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub scope: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthSession {
    pub user: DiscordUser,
    pub token: AuthToken,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerStats {
    pub total_servers: u32,
    pub total_users: u32,
    pub messages_today: u64,
    pub infractions_today: u32,
    pub bots_online: u32,
    pub bots_total: u32,
    pub workers_online: u32,
    pub workers_total: u32,
    pub postgres_online: bool,
    pub redis_online: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub bot: String,
    pub server: String,
    pub message: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub details: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Infraction {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub server: String,
    pub infraction_type: String,
    pub reason: String,
    pub created_at: String,
    pub moderator: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfirmedBan {
    pub id: String,
    pub guild_id: String,
    pub target_id: String,
    pub target_name: String,
    pub moderator_name: String,
    pub action_type: String,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModerationRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub rule_type: String,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateRuleParams {
    pub guild_id: String,
    pub flag_type: String,
    pub weight: f64,
    pub threshold_warn: f64,
    pub threshold_delete: f64,
    pub threshold_mute: f64,
    pub threshold_ban: f64,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub assigned_to: Option<String>,
    pub server: String,
    pub category: String,
    pub ticket_type: Option<String>,
    pub channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
    pub invited_user_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub messages_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TicketMessage {
    pub id: String,
    pub ticket_id: String,
    pub author_name: String,
    pub author_role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TicketDetail {
    pub ticket: Ticket,
    pub messages: Vec<TicketMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityEvent {
    pub id: String,
    pub guild_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub user_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModerationActionRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub duration: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModerationActionResponse {
    pub id: String,
    pub action_type: String,
    pub target_name: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserModerationHistory {
    pub target_id: String,
    pub target_name: String,
    pub total_warns: u32,
    pub total_mutes: u32,
    pub total_bans: u32,
    pub actions: Vec<ModerationActionResponse>,
}

// ── Voice Channels ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceChannel {
    pub id: String,
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
    pub locked: bool,
    pub member_limit: Option<i32>,
    pub status: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceChannelCoAdmin {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub granted_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceChannelBan {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceChannelDetail {
    pub channel: VoiceChannel,
    pub co_admins: Vec<VoiceChannelCoAdmin>,
    pub bans: Vec<VoiceChannelBan>,
}

// ── Role Panels ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RolePanel {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: Option<String>,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub max_roles: Option<i32>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RolePanelEntry {
    pub id: String,
    pub role_id: String,
    pub role_name: String,
    pub emoji: Option<String>,
    pub label: String,
    pub style: String,
    pub position: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RolePanelDetail {
    pub panel: RolePanel,
    pub entries: Vec<RolePanelEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoRoleConfig {
    pub id: String,
    pub guild_id: String,
    pub role_id: String,
    pub role_name: String,
    pub delay_secs: i32,
    pub enabled: bool,
}

// ── Top Users ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopUser {
    pub user_id: String,
    pub username: String,
    pub message_count: u64,
    pub voice_seconds: u64,
    pub voice_hours: f64,
}

// ── Dashboard Charts ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyActivity {
    pub day: String,
    pub messages: i64,
    pub voice_minutes: i64,
    pub active_members: i32,
    pub new_members: i32,
    pub leaves: i32,
    pub infractions: i32,
    pub warns: i32,
    pub mutes: i32,
    pub bans: i32,
}

// ── Levels / XP ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelConfig {
    pub guild_id: String,
    pub xp_per_message: i32,
    pub xp_per_voice_minute: i32,
    pub xp_cooldown_secs: i32,
    pub level_up_channel_id: Option<String>,
    pub level_up_message: String,
    pub excluded_channels: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserLevel {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub xp: i64,
    pub level: i32,
    pub xp_current: i64,
    pub xp_needed: i64,
    pub last_xp_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelReward {
    pub id: String,
    pub guild_id: String,
    pub level: i32,
    pub role_id: String,
}

// ── Audit Logs ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditLog {
    pub id: String,
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub details: serde_json::Value,
    pub created_at: String,
}

// ── Watched Users (Surveillance) ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WatchedUser {
    pub user_id: String,
    pub username: String,
    pub guild_id: String,
    pub guild_name: String,
    pub risk_level: String,
    pub total_warns: i64,
    pub total_mutes: i64,
    pub total_bans: i64,
    pub conduct_points: Option<i32>,
    pub max_conduct_points: Option<i32>,
    pub last_incident_at: Option<String>,
    pub security_events_count: i64,
    pub first_seen_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserDossier {
    pub user: WatchedUser,
    pub infractions: Vec<Infraction>,
    pub moderation_actions: Vec<ModerationActionResponse>,
    pub security_events: Vec<SecurityEvent>,
    pub conduct_log: Vec<ConductPointsLog>,
}

// ── Conduct ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConductConfig {
    pub guild_id: String,
    pub max_points: i32,
    pub regen_amount: i32,
    pub regen_interval: String,
    pub penalty_warn: i32,
    pub penalty_delete: i32,
    pub penalty_mute: i32,
    pub penalty_ban: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserConductPoints {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub points: i32,
    pub last_regen_at: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConductPointsLog {
    pub id: String,
    pub delta: i32,
    pub reason: String,
    pub points_before: i32,
    pub points_after: i32,
    pub created_at: String,
}

// ── Discord Roles ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscordRole {
    pub id: String,
    pub guild_id: String,
    pub name: String,
    pub color: i32,
    pub position: i32,
    pub permissions: String,
    pub mentionable: bool,
    pub managed: bool,
    pub icon: Option<String>,
    pub member_count: i32,
    pub synced_at: String,
}

use std::future::Future;
use std::pin::Pin;

use super::entities::{
    BotDefinition, BotGuildConfig, ConductConfig, ConductPointsLog, Guild, Infraction, LogEntry,
    ModerationActionRequest, ModerationActionResponse, ModerationRule, SecurityEvent, ServerStats,
    AuditLog, AutoRoleConfig, DailyActivity, LevelConfig, LevelReward, RolePanel, RolePanelDetail,
    Ticket, TicketDetail, UpdateRuleParams, UserConductPoints, UserDossier,
    UserLevel, UserModerationHistory, VoiceChannel, VoiceChannelDetail, WatchedUser,
};

type BoxFut<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send>>;

pub trait GuildRepository: Send + Sync + 'static {
    fn get_guilds(&self) -> BoxFut<Vec<Guild>>;
}

pub trait BotConfigRepository: Send + Sync + 'static {
    fn get_definitions(&self) -> BoxFut<Vec<BotDefinition>>;
    fn get_guild_config(&self, guild_id: String) -> BoxFut<Vec<BotGuildConfig>>;
    fn set_config(&self, guild_id: String, bot_name: String, key: String, value: String) -> BoxFut<()>;
    fn delete_config(&self, guild_id: String, bot_name: String, key: String) -> BoxFut<()>;
}

pub trait StatsRepository: Send + Sync + 'static {
    fn get_dashboard_stats(&self) -> BoxFut<ServerStats>;
}

pub trait LogsRepository: Send + Sync + 'static {
    fn get_logs(&self, guild_id: Option<String>) -> BoxFut<Vec<LogEntry>>;
}

pub trait InfractionsRepository: Send + Sync + 'static {
    fn get_infractions(&self, guild_id: Option<String>) -> BoxFut<Vec<Infraction>>;
}

pub trait RulesRepository: Send + Sync + 'static {
    fn get_rules(&self, guild_id: Option<String>) -> BoxFut<Vec<ModerationRule>>;
    fn toggle_rule(&self, id: String, enabled: bool) -> BoxFut<bool>;
    fn update_rule(&self, params: UpdateRuleParams) -> BoxFut<()>;
}

pub trait TicketsRepository: Send + Sync + 'static {
    fn get_tickets(&self) -> BoxFut<Vec<Ticket>>;
    fn get_ticket_detail(&self, id: String) -> BoxFut<TicketDetail>;
    fn reply_ticket(&self, ticket_id: String, content: String) -> BoxFut<()>;
    fn close_ticket(&self, id: String) -> BoxFut<()>;
    fn assign_ticket(&self, id: String, assignee: String) -> BoxFut<()>;
}

pub trait SecurityRepository: Send + Sync + 'static {
    fn get_events(&self, guild_id: Option<String>) -> BoxFut<Vec<SecurityEvent>>;
}

pub trait ModerationRepository: Send + Sync + 'static {
    fn log_action(&self, action: ModerationActionRequest) -> BoxFut<ModerationActionResponse>;
    fn get_history(&self, guild_id: String, user_id: String) -> BoxFut<UserModerationHistory>;
}

pub trait VoiceChannelRepository: Send + Sync + 'static {
    fn get_channels(&self, guild_id: String) -> BoxFut<Vec<VoiceChannel>>;
    fn get_channel_detail(&self, channel_id: String) -> BoxFut<VoiceChannelDetail>;
}

pub trait ConductRepository: Send + Sync + 'static {
    fn get_config(&self, guild_id: String) -> BoxFut<ConductConfig>;
    fn get_leaderboard(&self, guild_id: String) -> BoxFut<Vec<UserConductPoints>>;
    fn get_points(&self, guild_id: String, user_id: String) -> BoxFut<UserConductPoints>;
    fn get_log(&self, guild_id: String, user_id: String) -> BoxFut<Vec<ConductPointsLog>>;
}

pub trait RolePanelsRepository: Send + Sync + 'static {
    fn get_panels(&self, guild_id: String) -> BoxFut<Vec<RolePanel>>;
    fn get_panel(&self, panel_id: String) -> BoxFut<RolePanelDetail>;
    fn get_auto_roles(&self, guild_id: String) -> BoxFut<Vec<AutoRoleConfig>>;
}

pub trait DashboardChartsRepository: Send + Sync + 'static {
    fn get_activity_trend(&self, guild_id: Option<String>, days: Option<i32>) -> BoxFut<Vec<DailyActivity>>;
}

pub trait LevelRepository: Send + Sync + 'static {
    fn get_level_config(&self, guild_id: String) -> BoxFut<LevelConfig>;
    fn get_level_leaderboard(&self, guild_id: String) -> BoxFut<Vec<UserLevel>>;
    fn get_level_rewards(&self, guild_id: String) -> BoxFut<Vec<LevelReward>>;
}

pub trait AuditLogRepository: Send + Sync + 'static {
    fn get_audit_logs(&self, guild_id: Option<String>, event_type: Option<String>, limit: Option<i64>) -> BoxFut<Vec<AuditLog>>;
}

pub trait WatchedUsersRepository: Send + Sync + 'static {
    fn get_watched_users(&self, guild_id: Option<String>) -> BoxFut<Vec<WatchedUser>>;
    fn get_user_dossier(&self, guild_id: String, user_id: String) -> BoxFut<UserDossier>;
}

pub trait AppAdapter:
    GuildRepository
    + BotConfigRepository
    + StatsRepository
    + LogsRepository
    + InfractionsRepository
    + RulesRepository
    + TicketsRepository
    + SecurityRepository
    + ModerationRepository
    + VoiceChannelRepository
    + ConductRepository
    + WatchedUsersRepository
    + AuditLogRepository
    + LevelRepository
    + DashboardChartsRepository
    + RolePanelsRepository
{
}

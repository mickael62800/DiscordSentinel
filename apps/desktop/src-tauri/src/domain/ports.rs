use std::future::Future;
use std::pin::Pin;

use super::entities::{
    BotDefinition, BotGuildConfig, ConfirmedBan, ConductConfig, ConductPointsLog, Guild, GuildMember, Infraction, LogEntry,
    Member, MemberSummary, ModerationActionRequest, ModerationActionResponse, ModerationRule, SecurityEvent, ServerStats,
    AuditLog, AutoRoleConfig, DailyActivity, LevelConfig, LevelReward, RolePanel, RolePanelDetail,
    Ticket, TicketDetail, UpdateRuleParams, UserConductPoints, UserDossier,
    TopUser, UserLevel, UserModerationHistory, VoiceChannel, VoiceChannelDetail, WatchedUser, DiscordRole,
    CoudeCombat, CoudePlayer,
};

type BoxFut<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send>>;

pub trait GuildRepository: Send + Sync + 'static {
    fn get_guilds(&self) -> BoxFut<Vec<Guild>>;
    fn get_guild_members(&self, guild_id: String) -> BoxFut<Vec<GuildMember>>;
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
    fn delete_logs_by_category(&self, category: String) -> BoxFut<()>;
}

pub trait InfractionsRepository: Send + Sync + 'static {
    fn get_infractions(&self, guild_id: Option<String>) -> BoxFut<Vec<Infraction>>;
    fn delete_infraction(&self, id: String) -> BoxFut<()>;
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
    fn get_confirmed_bans(&self, guild_id: Option<String>) -> BoxFut<Vec<ConfirmedBan>>;
    fn execute_ban(&self, guild_id: String, user_id: String, reason: String) -> BoxFut<()>;
    fn execute_unban(&self, guild_id: String, user_id: String) -> BoxFut<()>;
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
    fn adjust_points(&self, guild_id: String, user_id: String, amount: i32, reason: String) -> BoxFut<UserConductPoints>;
}

pub trait RolePanelsRepository: Send + Sync + 'static {
    fn get_panels(&self, guild_id: String) -> BoxFut<Vec<RolePanel>>;
    fn get_panel(&self, panel_id: String) -> BoxFut<RolePanelDetail>;
    fn get_auto_roles(&self, guild_id: String) -> BoxFut<Vec<AutoRoleConfig>>;
}

pub trait DashboardChartsRepository: Send + Sync + 'static {
    fn get_activity_trend(&self, guild_id: Option<String>, days: Option<i32>) -> BoxFut<Vec<DailyActivity>>;
    fn get_top_users(&self, guild_id: String, limit: Option<u32>) -> BoxFut<Vec<TopUser>>;
}

pub trait LevelRepository: Send + Sync + 'static {
    fn get_level_config(&self, guild_id: String) -> BoxFut<LevelConfig>;
    fn get_level_leaderboard(&self, guild_id: String) -> BoxFut<Vec<UserLevel>>;
    fn get_level_rewards(&self, guild_id: String) -> BoxFut<Vec<LevelReward>>;
    fn set_level_reward(&self, guild_id: String, level: i32, role_id: String, source: String) -> BoxFut<LevelReward>;
    fn delete_level_reward(&self, guild_id: String, level: i32, source: String) -> BoxFut<()>;
}

pub trait AuditLogRepository: Send + Sync + 'static {
    fn get_audit_logs(&self, guild_id: Option<String>, event_type: Option<String>, limit: Option<i64>) -> BoxFut<Vec<AuditLog>>;
}

pub trait WatchedUsersRepository: Send + Sync + 'static {
    fn get_watched_users(&self, guild_id: Option<String>) -> BoxFut<Vec<WatchedUser>>;
    fn get_user_dossier(&self, guild_id: String, user_id: String) -> BoxFut<UserDossier>;
    fn remove_watched_user(&self, guild_id: String, user_id: String) -> BoxFut<()>;
}

pub trait DiscordRolesRepository: Send + Sync + 'static {
    fn get_discord_roles(&self, guild_id: String) -> BoxFut<Vec<DiscordRole>>;
    fn create_discord_role(&self, guild_id: String, name: String, color: u32, permissions: Option<String>) -> BoxFut<serde_json::Value>;
    fn edit_discord_role(&self, guild_id: String, role_id: String, name: Option<String>, color: Option<u32>, permissions: Option<String>, mentionable: Option<bool>, hoist: Option<bool>) -> BoxFut<serde_json::Value>;
    fn delete_discord_role(&self, guild_id: String, role_id: String) -> BoxFut<()>;
}

pub trait MembersRepository: Send + Sync + 'static {
    fn get_members(&self, guild_id: String) -> BoxFut<Vec<Member>>;
    fn get_member_summary(&self, guild_id: String, user_id: String) -> BoxFut<MemberSummary>;
}

pub trait CoudeRepository: Send + Sync + 'static {
    fn get_combats(&self, guild_id: String, status: Option<String>) -> BoxFut<Vec<CoudeCombat>>;
    fn get_players(&self, guild_id: String) -> BoxFut<Vec<CoudePlayer>>;
    fn cancel_combat(&self, combat_id: String) -> BoxFut<()>;
    fn adjust_coins(&self, guild_id: String, user_id: String, amount: i64) -> BoxFut<()>;
}

/// Phase 7 B — Gestion RBAC fin des users d'une guild (backed par les
/// endpoints `/api/rbac/*` cote API).
pub trait RbacRepository: Send + Sync + 'static {
    fn list_guild_users(
        &self,
        guild_id: String,
    ) -> BoxFut<Vec<super::entities::GuildUserRole>>;

    fn get_my_role(&self, guild_id: String) -> BoxFut<super::entities::MyRole>;

    fn grant_role(
        &self,
        guild_id: String,
        user_id: String,
        role: String,
        display_name: Option<String>,
    ) -> BoxFut<()>;

    fn update_role(&self, guild_id: String, user_id: String, role: String) -> BoxFut<()>;

    fn revoke_role(&self, guild_id: String, user_id: String) -> BoxFut<()>;
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
    + DiscordRolesRepository
    + MembersRepository
    + CoudeRepository
    + RbacRepository
{
}

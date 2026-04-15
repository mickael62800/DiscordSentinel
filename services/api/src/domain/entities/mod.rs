mod bot_config;
mod dashboard_stats;
mod guild;
mod infraction;
mod log_entry;
mod message_analysis;
mod moderation_action;
mod rule;
mod security_event;
mod ticket;
mod user_stats;
mod conduct;
mod voice_channel;
mod watched_user;
mod audit_log;
mod image_analysis;
mod level;
mod daily_activity;
mod role_panel;
mod discord_role;
mod ia_config;
mod strikes;
mod sanction_reminder;
mod user_note;
pub mod analytics;

pub use ia_config::IaConfig;
pub use audit_log::AuditLog;
pub use image_analysis::{ImageAnalysis, ImageClassification};
pub use bot_config::{BotDefinition, BotGuildConfig};
pub use conduct::{ConductConfig, ConductPointsLog, UserConductPoints};
pub use daily_activity::DailyActivity;
pub use role_panel::{AutoRole, RolePanel, RolePanelDetail, RolePanelEntry};
pub use dashboard_stats::DashboardStats;
pub use guild::Guild;
pub use infraction::Infraction;
pub use level::{xp_progress, xp_for_level, level_from_xp, LevelConfig, LevelReward, UserLevel, XpSource};
pub use log_entry::LogEntry;
pub use message_analysis::MessageAnalysis;
pub use moderation_action::{ModerationAction, UserModerationHistory};
pub use rule::Rule;
pub use security_event::SecurityEvent;
pub use ticket::{Ticket, TicketDetail, TicketMessage};
pub use user_stats::{GuildStatsOverview, GuildVoiceStats, UserStats, VoiceSessionStats};
pub use voice_channel::{
    VoiceChannel, VoiceChannelBan, VoiceChannelCoAdmin, VoiceChannelDetail,
    VoiceChannelInviteLink, VoiceChannelTheme, VoiceChannelWhitelistEntry,
};
pub use watched_user::WatchedUser;
pub use discord_role::DiscordRole;
pub use strikes::{StrikeConfig, StrikeResult, StrikeThreshold, UserStrike};
pub use sanction_reminder::SanctionReminder;
pub use user_note::UserNote;

mod user_activity;
pub use user_activity::UserActivity;

mod guild_member;
pub use guild_member::{GuildMember, MemberSummary, MemberConduct, MemberInfractions, MemberModeration, MemberStats};

mod blackjack;
pub use blackjack::{BlackjackGame, Card, calculate_score, create_deck};

mod wallet;
pub use wallet::{Wallet, WalletTransaction};

mod coude_player;
pub use coude_player::{
    title_for_level as coude_title_for_level, xp_for_level as coude_xp_for_level, CombatStat,
    CoudePlayer, XpProgress, COUDE_MAX_LEVEL,
};

mod coude_combat;
pub use coude_combat::{CombatResolution, CoudeCombat, NewCoudeCombat};

mod coude_bet;
pub use coude_bet::{
    calculate_bet_resolution, BetPayout, BetResolutionPlan, CoudeBet,
    FighterBetBonus as CoudeFighterBetBonus, NewCoudeBet, RefundSummary,
};

mod coude_inventory;
pub use coude_inventory::{
    CoudeInsurance, CoudeInventoryItem, CoudePrime, NewCoudePrime,
};

mod coude_social;
pub use coude_social::{
    CoudeCurrentSeason, CoudeEvent, CoudeLeaderboardEntry, LeaderboardCategory, NewDailyChaos,
};

mod coude_cashbox;
pub use coude_cashbox::{
    CashboxRedistribution, CashboxRedistributionEntry, CashboxSource, CoudeCashbox,
};

mod coude_steal_protection;
pub use coude_steal_protection::{
    find_protection_item, CoudeStealProtection, StealProtectionDuration, StealProtectionItemDef,
    STEAL_PROTECTION_ITEMS,
};

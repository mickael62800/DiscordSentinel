//! Test helpers : construit un AppState complet avec des stubs pour tous les traits.
//! Seul le use case sous test est fonctionnel, les autres panic si appeles.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_api::adapters::inbound::ws::broadcaster::EventBroadcaster;
use sentinel_api::adapters::outbound::job_client::JobClient;
use sentinel_core::domain::entities::ai::image_analysis::*;
use sentinel_core::domain::entities::ai::message_analysis::*;
use sentinel_core::domain::entities::audit::audit_log::*;
use sentinel_core::domain::entities::audit::dashboard_stats::*;
use sentinel_core::domain::entities::audit::security_event::*;
use sentinel_core::domain::entities::audit::user_activity::*;
use sentinel_core::domain::entities::audit::user_stats::*;
use sentinel_core::domain::entities::audit::watched_user::*;
use sentinel_core::domain::entities::casino::blackjack::*;
use sentinel_core::domain::entities::casino::slot::*;
use sentinel_core::domain::entities::casino::wallet::*;
use sentinel_core::domain::entities::casino::wheel::*;
use sentinel_core::domain::entities::community::daily_activity::*;
use sentinel_core::domain::entities::community::guild_member::*;
use sentinel_core::domain::entities::community::level::*;
use sentinel_core::domain::entities::community::role_panel::*;
use sentinel_core::domain::entities::community::voice_channel::*;
use sentinel_core::domain::entities::coude::bet::*;
use sentinel_core::domain::entities::coude::cashbox::*;
use sentinel_core::domain::entities::coude::combat::*;
use sentinel_core::domain::entities::coude::heist::*;
use sentinel_core::domain::entities::coude::inventory::*;
use sentinel_core::domain::entities::coude::player::*;
use sentinel_core::domain::entities::coude::social::*;
use sentinel_core::domain::entities::coude::steal::*;
use sentinel_core::domain::entities::coude::steal::*;
use sentinel_core::domain::entities::coude::taunt::*;
use sentinel_core::domain::entities::moderation::infraction::*;
use sentinel_core::domain::entities::moderation::action::applied::*;
use sentinel_core::domain::entities::moderation::action::sanction_reminder::*;
use sentinel_core::domain::entities::moderation::action::strikes::*;
use sentinel_core::domain::entities::moderation::user_note::*;
use sentinel_core::domain::entities::system::*;
use sentinel_core::domain::entities::system::bot_config::*;
use sentinel_core::domain::entities::system::discord_role::*;
use sentinel_core::domain::entities::system::guild::*;
use sentinel_core::domain::entities::system::log_entry::*;
use sentinel_core::domain::entities::system::rule::*;
use sentinel_core::domain::entities::system::ticket::*;
use sentinel_core::domain::entities::system::analytics::*;
use sentinel_core::domain::errors::DomainError;
use sentinel_api::adapters::outbound::discord_api::DiscordApi;
use sentinel_api::adapters::outbound::discord_api::DiscordApiService;
use sentinel_api::adapters::outbound::discord_api::DiscordChannel;
use sentinel_api::adapters::outbound::discord_api::DiscordMember;
use sentinel_api::adapters::outbound::discord_api::DiscordUser;
use sentinel_api::adapters::outbound::discord_api::UserGuild;
use sentinel_api::ports::inbound::ai::analyze_image::*;
use sentinel_api::ports::inbound::ai::analyze_message::*;
use sentinel_api::ports::inbound::audit::*;
use sentinel_api::ports::inbound::audit::manage_audit_logs::*;
use sentinel_api::ports::inbound::audit::manage_security::*;
use sentinel_api::ports::inbound::audit::manage_stats::*;
use sentinel_api::ports::inbound::audit::manage_watched_users::*;
use sentinel_api::ports::inbound::casino::*;
use sentinel_api::ports::inbound::community::*;
use sentinel_api::ports::inbound::community::manage_levels::*;
use sentinel_api::ports::inbound::community::manage_members::*;
use sentinel_api::ports::inbound::community::manage_role_panels::*;
use sentinel_api::ports::inbound::community::manage_voice_channels::*;
use sentinel_api::ports::inbound::coude::*;
use sentinel_api::ports::inbound::coude::manage_bets::*;
use sentinel_api::ports::inbound::coude::manage_steal_protections::*;
use sentinel_api::ports::inbound::coude::resolve_friendly_duel::*;
use sentinel_api::ports::inbound::moderation::*;
use sentinel_api::ports::inbound::moderation::manage_infractions::*;
use sentinel_api::ports::inbound::moderation::manage_moderation::*;
use sentinel_api::ports::inbound::moderation::manage_notes::*;
use sentinel_api::ports::inbound::moderation::manage_reminders::*;
use sentinel_api::ports::inbound::moderation::manage_rules::*;
use sentinel_api::ports::inbound::moderation::manage_strikes::*;
use sentinel_api::ports::inbound::system::manage_tickets::*;
use sentinel_api::ports::outbound::audit::analytics_repository::*;
use sentinel_api::ports::outbound::audit::modstats_repository::*;
use sentinel_api::ports::outbound::audit::user_activity_repository::*;
use sentinel_api::ports::outbound::casino::blackjack_repository::*;
use sentinel_api::ports::outbound::casino::blackjack_table_repository::*;
use sentinel_api::ports::outbound::casino::game_repository::*;
use sentinel_api::ports::outbound::casino::wallet_repository::*;
use sentinel_api::ports::outbound::community::daily_activity_repository::*;
use sentinel_api::ports::outbound::community::discord_role_repository::*;
use sentinel_api::ports::outbound::community::temp_role_repository::*;
use sentinel_api::ports::outbound::community::welcome_config_repository::*;
use sentinel_api::ports::outbound::coude::flavor_templates_repository::*;
use sentinel_api::ports::outbound::coude::sponsorship_repository::*;
use sentinel_api::ports::outbound::moderation::evidence_repository::*;
use sentinel_api::ports::outbound::moderation::pending_action_repository::*;
use sentinel_api::ports::outbound::moderation::review_repository::*;
use sentinel_api::ports::outbound::system::bot_config_repository::*;
use sentinel_api::ports::outbound::system::guild_repository::*;
use sentinel_api::ports::outbound::system::log_repository::*;

// Chaque fichier de test d'intégration est compilé comme une crate séparée.
// Du coup Rust voit les helpers comme "unused" dans les tests qui n'en
// consomment qu'une partie — d'où les `#[allow(dead_code)]` ciblés plus bas.

// ══════════════════════════════════════════════════════════
// Stub Use Cases (inbound)
// ══════════════════════════════════════════════════════════

pub struct StubAnalyzeMessage;
#[async_trait]
impl AnalyzeMessageUseCase for StubAnalyzeMessage {
    async fn analyze(&self, _: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> { unimplemented!() }
}

pub struct StubAnalyzeImage;
#[async_trait]
impl AnalyzeImageUseCase for StubAnalyzeImage {
    async fn analyze_image(&self, _: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> { unimplemented!() }
}

pub struct StubRules;
#[async_trait]
impl ManageRulesUseCase for StubRules {
    async fn get_rules(&self, _: &str) -> Result<Vec<Rule>, DomainError> { unimplemented!() }
    async fn get_all_rules(&self) -> Result<Vec<Rule>, DomainError> { unimplemented!() }
    async fn toggle_rule(&self, _: Uuid, _: bool) -> Result<bool, DomainError> { unimplemented!() }
    async fn create_or_update_rule(&self, _: CreateRuleCommand) -> Result<Rule, DomainError> { unimplemented!() }
    async fn delete_rule(&self, _: &str, _: Uuid) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubInfractions;
#[async_trait]
impl ManageInfractionsUseCase for StubInfractions {
    async fn list_infractions(&self, _: &str, _: InfractionFilters) -> Result<Vec<Infraction>, DomainError> { unimplemented!() }
    async fn list_all_infractions(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> { unimplemented!() }
    async fn count_today(&self) -> Result<u64, DomainError> { unimplemented!() }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> { unimplemented!() }
    async fn delete_infraction(&self, _: &str) -> Result<bool, DomainError> { unimplemented!() }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> { unimplemented!() }
}

pub struct StubTickets;
#[async_trait]
impl ManageTicketsUseCase for StubTickets {
    async fn list_tickets(&self, _: Option<String>, _: Option<String>, _: Option<String>, _: Option<String>, _: i64, _: i64) -> Result<Vec<Ticket>, DomainError> { unimplemented!() }
    async fn get_ticket_detail(&self, _: &str) -> Result<TicketDetail, DomainError> { unimplemented!() }
    async fn create_ticket(&self, _: CreateTicketCommand) -> Result<Ticket, DomainError> { unimplemented!() }
    async fn reply_ticket(&self, _: ReplyTicketCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn close_ticket(&self, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn assign_ticket(&self, _: AssignTicketCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn update_status(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn update_ticket_channel(&self, _: UpdateTicketChannelCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn update_priority(&self, _: Uuid, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn update_sla(&self, _: Uuid, _: Option<&str>, _: Option<&str>, _: Option<i32>) -> Result<(), DomainError> { Ok(()) }
}

pub struct StubSecurity;
#[async_trait]
impl ManageSecurityUseCase for StubSecurity {
    async fn report_event(&self, _: ReportSecurityEventCommand) -> Result<SecurityEvent, DomainError> { unimplemented!() }
    async fn list_events(&self, _: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> { unimplemented!() }
    async fn analyze_new_member(&self, _: AnalyzeNewMemberCommand) -> Result<SecurityDecision, DomainError> { unimplemented!() }
}

pub struct StubModeration;
#[async_trait]
impl ManageModerationUseCase for StubModeration {
    async fn list_actions(&self, _: Option<&str>, _: i64) -> Result<Vec<ModerationAction>, DomainError> { unimplemented!() }
    async fn log_action(&self, _: LogModerationCommand) -> Result<ModerationAction, DomainError> { unimplemented!() }
    async fn get_history(&self, _: &str, _: &str) -> Result<UserModerationHistory, DomainError> { unimplemented!() }
    async fn list_bans(&self, _: Option<&str>, _: i64, _: i64) -> Result<Vec<ModerationAction>, DomainError> { unimplemented!() }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn delete_action(&self, _: Uuid) -> Result<bool, DomainError> { unimplemented!() }
}

pub struct StubStats;
#[async_trait]
impl ManageStatsUseCase for StubStats {
    async fn record_messages(&self, _: manage_stats::RecordMessagesCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn record_voice(&self, _: manage_stats::RecordVoiceCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn get_user_stats(&self, _: &str, _: &str) -> Result<Option<UserStats>, DomainError> { unimplemented!() }
    async fn get_guild_overview(&self, _: &str) -> Result<GuildStatsOverview, DomainError> { unimplemented!() }
    async fn get_leaderboard(&self, _: &str, _: u32) -> Result<Vec<UserStats>, DomainError> { unimplemented!() }
    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> { unimplemented!() }
    async fn get_guild_voice_stats(&self, _: &str, _: u32, _: u32) -> Result<GuildVoiceStats, DomainError> { unimplemented!() }
}


pub struct StubWatchedUsers;
#[async_trait]
impl ManageWatchedUsersUseCase for StubWatchedUsers {
    async fn list_watched_users(&self, _: Option<&str>, _: i64, _: i64) -> Result<Vec<WatchedUser>, DomainError> { unimplemented!() }
    async fn get_user_dossier(&self, _: &str, _: &str) -> Result<manage_watched_users::UserDossier, DomainError> { unimplemented!() }
    async fn add_manual_watch(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn remove_manual_watch(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubAuditLogs;
#[async_trait]
impl ManageAuditLogsUseCase for StubAuditLogs {
    async fn create(&self, cmd: manage_audit_logs::CreateAuditLogCommand) -> Result<AuditLog, DomainError> {
        Ok(AuditLog {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            event_type: cmd.event_type,
            actor_id: cmd.actor_id,
            actor_name: cmd.actor_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            channel_id: cmd.channel_id,
            channel_name: cmd.channel_name,
            details: cmd.details,
            created_at: chrono::Utc::now(),
        })
    }
    async fn list(&self, _: Option<&str>, _: manage_audit_logs::AuditLogFilters) -> Result<Vec<AuditLog>, DomainError> { unimplemented!() }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> { unimplemented!() }
}

pub struct StubLevels;
#[async_trait]
impl ManageLevelsUseCase for StubLevels {
    async fn get_config(&self, _: &str) -> Result<LevelConfig, DomainError> { unimplemented!() }
    async fn save_config(&self, _: manage_levels::SaveLevelConfigCommand) -> Result<LevelConfig, DomainError> { unimplemented!() }
    async fn add_xp(&self, _: manage_levels::AddXpCommand) -> Result<manage_levels::AddXpResult, DomainError> { unimplemented!() }
    async fn get_user_level(&self, _: &str, _: &str) -> Result<UserLevel, DomainError> { unimplemented!() }
    async fn get_leaderboard(&self, _: &str, _: i64) -> Result<Vec<UserLevel>, DomainError> { unimplemented!() }
    async fn get_leaderboard_by_source(&self, _: &str, _: XpSource, _: i64) -> Result<Vec<UserLevel>, DomainError> { unimplemented!() }
    async fn set_user_xp(&self, _: manage_levels::SetUserXpCommand) -> Result<UserLevel, DomainError> { unimplemented!() }
    async fn reset_user_xp(&self, _: &str, _: &str, _: manage_levels::ResetTarget) -> Result<UserLevel, DomainError> { unimplemented!() }
}

#[allow(dead_code)]
pub struct StubAnnouncements;
#[async_trait]
impl sentinel_api::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase for StubAnnouncements {
    async fn create(&self, _: sentinel_api::ports::inbound::community::manage_announcements::CreateAnnouncementCommand) -> Result<sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement, DomainError> { unimplemented!() }
    async fn update(&self, _: sentinel_api::ports::inbound::community::manage_announcements::UpdateAnnouncementCommand) -> Result<sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement, DomainError> { unimplemented!() }
    async fn delete(&self, _: uuid::Uuid) -> Result<(), DomainError> { unimplemented!() }
    async fn get(&self, _: uuid::Uuid) -> Result<sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement, DomainError> { unimplemented!() }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement>, DomainError> { unimplemented!() }
    async fn toggle(&self, _: uuid::Uuid, _: bool) -> Result<bool, DomainError> { unimplemented!() }
    async fn fetch_due_and_prepare(&self, _: chrono::DateTime<chrono::Utc>, _: i64) -> Result<Vec<sentinel_api::ports::inbound::community::manage_announcements::RenderedAnnouncement>, DomainError> { unimplemented!() }
    async fn record_run_result(&self, _: uuid::Uuid, _: Vec<sentinel_core::domain::entities::community::announcement::ChannelPostResult>) -> Result<(), DomainError> { unimplemented!() }
    async fn preview(&self, _: uuid::Uuid) -> Result<sentinel_api::ports::inbound::community::manage_announcements::RenderedAnnouncement, DomainError> { unimplemented!() }
    async fn list_runs(&self, _: uuid::Uuid, _: i64) -> Result<Vec<sentinel_core::domain::entities::community::announcement::AnnouncementRun>, DomainError> { unimplemented!() }
    async fn record_button_interaction(&self, _: uuid::Uuid, _: Option<uuid::Uuid>, _: String, _: Option<String>, _: String, _: Option<String>) -> Result<(), DomainError> { unimplemented!() }
    async fn list_button_interactions(&self, _: uuid::Uuid, _: i64) -> Result<Vec<sentinel_core::domain::entities::community::announcement::ButtonInteraction>, DomainError> { unimplemented!() }
}

#[allow(dead_code)]
pub struct StubConfessions;
#[async_trait]
impl sentinel_api::ports::inbound::community::manage_confessions::ManageConfessionsUseCase for StubConfessions {
    async fn create(&self, _: sentinel_api::ports::inbound::community::manage_confessions::CreateConfessionCommand) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError> { unimplemented!() }
    async fn update_message_refs(&self, _: uuid::Uuid, _: String, _: String, _: Option<String>) -> Result<(), DomainError> { unimplemented!() }
    async fn edit_content(&self, _: uuid::Uuid, _: &str, _: String) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError> { unimplemented!() }
    async fn delete(&self, _: uuid::Uuid, _: String, _: Option<String>) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError> { unimplemented!() }
    async fn get(&self, _: uuid::Uuid) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError> { unimplemented!() }
    async fn get_by_message_id(&self, _: &str) -> Result<Option<sentinel_core::domain::entities::community::confession::Confession>, DomainError> { unimplemented!() }
    async fn get_by_public_number(&self, _: &str, _: i32) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError> { unimplemented!() }
    async fn list(&self, _: &str, _: i64, _: bool) -> Result<Vec<sentinel_core::domain::entities::community::confession::Confession>, DomainError> { unimplemented!() }
    async fn create_reply(&self, _: sentinel_api::ports::inbound::community::manage_confessions::CreateReplyCommand) -> Result<sentinel_core::domain::entities::community::confession::ConfessionReply, DomainError> { unimplemented!() }
    async fn update_reply_message_id(&self, _: uuid::Uuid, _: String) -> Result<(), DomainError> { unimplemented!() }
    async fn delete_reply(&self, _: uuid::Uuid, _: String) -> Result<sentinel_core::domain::entities::community::confession::ConfessionReply, DomainError> { unimplemented!() }
    async fn list_replies(&self, _: uuid::Uuid) -> Result<Vec<sentinel_core::domain::entities::community::confession::ConfessionReply>, DomainError> { unimplemented!() }
    async fn create_report(&self, _: sentinel_api::ports::inbound::community::manage_confessions::CreateReportCommand) -> Result<sentinel_core::domain::entities::community::confession::ConfessionReport, DomainError> { unimplemented!() }
    async fn list_reports(&self, _: &str, _: Option<sentinel_core::domain::entities::community::confession::ReportStatus>, _: i64) -> Result<Vec<sentinel_core::domain::entities::community::confession::ConfessionReport>, DomainError> { unimplemented!() }
    async fn resolve_report(&self, _: uuid::Uuid, _: sentinel_core::domain::entities::community::confession::ReportStatus, _: String) -> Result<(), DomainError> { unimplemented!() }
    async fn get_config(&self, _: &str) -> Result<sentinel_core::domain::entities::community::confession::ConfessionConfig, DomainError> { unimplemented!() }
    async fn save_config(&self, _: sentinel_core::domain::entities::community::confession::ConfessionConfig) -> Result<sentinel_core::domain::entities::community::confession::ConfessionConfig, DomainError> { unimplemented!() }
}

pub struct StubRolePanels;
#[async_trait]
impl ManageRolePanelsUseCase for StubRolePanels {
    async fn create_panel(&self, _: manage_role_panels::CreateRolePanelCommand) -> Result<RolePanelDetail, DomainError> { unimplemented!() }
    async fn get_panel(&self, _: &str) -> Result<RolePanelDetail, DomainError> { unimplemented!() }
    async fn get_panel_by_message(&self, _: &str) -> Result<Option<RolePanelDetail>, DomainError> { unimplemented!() }
    async fn list_panels(&self, _: &str) -> Result<Vec<RolePanel>, DomainError> { unimplemented!() }
    async fn set_message_id(&self, _: manage_role_panels::SetMessageIdCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn delete_panel(&self, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn list_auto_roles(&self, _: &str) -> Result<Vec<AutoRole>, DomainError> { unimplemented!() }
    async fn add_auto_role(&self, _: manage_role_panels::CreateAutoRoleCommand) -> Result<AutoRole, DomainError> { unimplemented!() }
    async fn delete_auto_role(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubReminders;
#[async_trait]
impl ManageRemindersUseCase for StubReminders {
    async fn create_reminder(&self, cmd: manage_reminders::CreateReminderCommand) -> Result<SanctionReminder, DomainError> {
        Ok(SanctionReminder {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            moderator_id: cmd.moderator_id,
            moderator_name: cmd.moderator_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            action_type: cmd.action_type,
            reason: cmd.reason,
            action_id: cmd.action_id,
            remind_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now(),
            status: "pending".into(),
            created_at: chrono::Utc::now(),
        })
    }
    async fn get_pending_reminders(&self) -> Result<Vec<SanctionReminder>, DomainError> { Ok(vec![]) }
    async fn mark_sent(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
    async fn cancel_for_action(&self, _: Uuid) -> Result<(), DomainError> { Ok(()) }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<SanctionReminder>, DomainError> { Ok(vec![]) }
}

pub struct StubNotes;
#[async_trait]
impl ManageNotesUseCase for StubNotes {
    async fn add_note(&self, _: manage_notes::AddNoteCommand) -> Result<UserNote, DomainError> { unimplemented!() }
    async fn get_notes(&self, _: &str, _: &str) -> Result<Vec<UserNote>, DomainError> { Ok(vec![]) }
    async fn delete_note(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
}

pub struct StubStrikes;
#[async_trait]
impl ManageStrikesUseCase for StubStrikes {
    async fn add_strike(&self, cmd: manage_strikes::AddStrikeCommand) -> Result<StrikeResult, DomainError> {
        Ok(StrikeResult {
            strike: UserStrike {
                id: Uuid::new_v4(),
                guild_id: cmd.guild_id,
                user_id: cmd.user_id,
                reason: cmd.reason,
                source: cmd.source,
                infraction_id: None,
                expires_at: None,
                created_at: chrono::Utc::now(),
            },
            active_count: 1,
            escalation_action: None,
            escalation_duration: None,
        })
    }
    async fn get_active_strikes(&self, _: &str, _: &str) -> Result<Vec<UserStrike>, DomainError> { Ok(vec![]) }
    async fn reset_strikes(&self, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn get_config(&self, guild_id: &str) -> Result<StrikeConfig, DomainError> { Ok(StrikeConfig::default_for_guild(guild_id)) }
    async fn save_config(&self, _: manage_strikes::SaveStrikeConfigCommand) -> Result<StrikeConfig, DomainError> { unimplemented!() }
}

// ══════════════════════════════════════════════════════════
// Stub Repositories (outbound)
// ══════════════════════════════════════════════════════════

pub struct StubAnalyticsRepo;
#[async_trait]
impl AnalyticsRepository for StubAnalyticsRepo {
    async fn get_heatmap(&self, _: Option<&str>, _: i32) -> Result<Vec<HourlyActivity>, DomainError> { unimplemented!() }
    async fn get_action_distribution(&self, _: Option<&str>, _: i32) -> Result<Vec<ActionDistribution>, DomainError> { unimplemented!() }
    async fn get_top_infractors(&self, _: Option<&str>, _: i32, _: i64) -> Result<Vec<TopInfractor>, DomainError> { unimplemented!() }
    async fn get_moderation_trend(&self, _: Option<&str>, _: i32) -> Result<Vec<ModerationTrend>, DomainError> { unimplemented!() }
    async fn get_peak_hours(&self, _: Option<&str>, _: i32) -> Result<Vec<PeakActivity>, DomainError> { unimplemented!() }
    async fn record_hourly(&self, _: &str, _: i16, _: i64, _: i32) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubDailyActivityRepo;
#[async_trait]
impl DailyActivityRepository for StubDailyActivityRepo {
    async fn get_activity(&self, _: Option<&str>, _: i32) -> Result<Vec<DailyActivity>, DomainError> { unimplemented!() }
    async fn record_daily_snapshot(&self, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubLogRepo;
#[async_trait]
impl LogRepository for StubLogRepo {
    async fn save(&self, _: &LogEntry) -> Result<(), DomainError> { Ok(()) }
    async fn find_all(&self, _: i64) -> Result<Vec<LogEntry>, DomainError> { Ok(vec![]) }
    async fn delete_by_category(&self, _: &str) -> Result<u64, DomainError> { Ok(0) }
    async fn delete_older_than_days(&self, _: i32) -> Result<u64, DomainError> { Ok(0) }
}

pub struct StubGuildRepo;
#[async_trait]
impl GuildRepository for StubGuildRepo {
    async fn upsert(&self, _: &Guild) -> Result<(), DomainError> { unimplemented!() }
    async fn find_all(&self) -> Result<Vec<Guild>, DomainError> { unimplemented!() }
    async fn find_by_id(&self, _: &str) -> Result<Option<Guild>, DomainError> { unimplemented!() }
    async fn delete(&self, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn delete_absent(&self, _: &[String]) -> Result<u64, DomainError> { unimplemented!() }
}

pub struct StubBotConfigRepo;
#[async_trait]
impl BotConfigRepository for StubBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> { unimplemented!() }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> { Ok(vec![]) }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> { unimplemented!() }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubDiscordRoleRepo;
#[async_trait]
impl DiscordRoleRepository for StubDiscordRoleRepo {
    async fn sync_roles(&self, _: &str, _: Vec<DiscordRole>) -> Result<(), DomainError> { unimplemented!() }
    async fn find_by_guild(&self, _: &str) -> Result<Vec<DiscordRole>, DomainError> { unimplemented!() }
    async fn find_by_id(&self, _: &str, _: &str) -> Result<Option<DiscordRole>, DomainError> { unimplemented!() }
}

pub struct StubMembers;
#[async_trait]
impl ManageMembersUseCase for StubMembers {
    async fn list_members(&self, _: &str) -> Result<Vec<GuildMember>, DomainError> { unimplemented!() }
    async fn get_member(&self, _: &str, _: &str) -> Result<GuildMember, DomainError> { unimplemented!() }
    async fn get_member_summary(&self, _: &str, _: &str) -> Result<MemberSummary, DomainError> { unimplemented!() }
    async fn sync_members(&self, _: manage_members::SyncMembersCommand) -> Result<u64, DomainError> { unimplemented!() }
    async fn register_member(&self, _: manage_members::RegisterMemberCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn remove_member(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn update_member(&self, _: manage_members::UpdateMemberCommand) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubWalletRepo;
#[async_trait]
impl WalletRepository for StubWalletRepo {
    async fn get_or_create(&self, _: &str, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn get(&self, _: &str, _: &str) -> Result<Option<Wallet>, DomainError> { unimplemented!() }
    async fn credit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn debit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn pay_combat_atomic(&self, _: &str, _: &str, _: i64, _: &str, _: i64, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn leaderboard(&self, _: &str, _: i64) -> Result<Vec<Wallet>, DomainError> { unimplemented!() }
    async fn get_transactions(&self, _: &str, _: &str, _: i64) -> Result<Vec<WalletTransaction>, DomainError> { unimplemented!() }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<Wallet>, DomainError> { unimplemented!() }
    async fn reset_wallet(&self, _: &str, _: &str, _: i64) -> Result<Wallet, DomainError> { unimplemented!() }
    async fn reset_all_wallets(&self, _: &str, _: i64) -> Result<u64, DomainError> { unimplemented!() }
}

pub struct StubBlackjackRepo;
#[async_trait]
impl BlackjackRepository for StubBlackjackRepo {
    async fn create(&self, _: &BlackjackGame) -> Result<(), DomainError> { unimplemented!() }
    async fn get_active(&self, _: &str, _: &str) -> Result<Option<BlackjackGame>, DomainError> { unimplemented!() }
    async fn update(&self, _: &BlackjackGame) -> Result<(), DomainError> { unimplemented!() }
    async fn get_by_id(&self, _: Uuid) -> Result<Option<BlackjackGame>, DomainError> { unimplemented!() }
    async fn list_by_guild(&self, _: &str, _: Option<&str>) -> Result<Vec<BlackjackGame>, DomainError> { Ok(vec![]) }
    async fn cancel_game(&self, _: Uuid) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubCoudeSocial;
#[async_trait]
impl manage_social::ManageCoudeSocialUseCase for StubCoudeSocial {
    async fn check_cooldown(&self, _: &str, _: &str, _: &str) -> Result<Option<chrono::DateTime<chrono::Utc>>, DomainError> { unimplemented!() }
    async fn set_cooldown(&self, _: &str, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn leaderboard(&self, _: &str, _: LeaderboardCategory, _: i64) -> Result<Vec<LeaderboardEntry>, DomainError> { unimplemented!() }
    async fn list_active_events(&self, _: &str) -> Result<Vec<Event>, DomainError> { unimplemented!() }
    async fn log_daily_chaos(&self, _: NewDailyChaos) -> Result<(), DomainError> { unimplemented!() }
    async fn current_season(&self, _: &str) -> Result<Season, DomainError> { unimplemented!() }
    async fn trigger_daily_chaos(&self, _: &str) -> Result<Option<DailyChaosOutcome>, DomainError> { unimplemented!() }
}

pub struct StubCoudeInventory;
#[async_trait]
impl manage_inventory::ManageCoudeInventoryUseCase for StubCoudeInventory {
    async fn list_inventory(&self, _: &str, _: &str) -> Result<Vec<InventoryItem>, DomainError> { unimplemented!() }
    async fn add_item(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn use_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { unimplemented!() }
    async fn has_item(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { unimplemented!() }
    async fn create_prime(&self, _: NewCoudePrime) -> Result<Prime, DomainError> { unimplemented!() }
    async fn list_active_primes(&self, _: &str, _: &str) -> Result<Vec<Prime>, DomainError> { unimplemented!() }
    async fn claim_primes(&self, _: &str, _: &str, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn buy_insurance(&self, _: &str, _: &str, _: bool, _: i64) -> Result<bool, DomainError> { unimplemented!() }
    async fn get_active_insurance(&self, _: &str, _: &str) -> Result<Option<Insurance>, DomainError> { unimplemented!() }
    async fn expire_insurance(&self, _: Uuid) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubCoudeEconomy;
#[async_trait]
impl manage_economy::ManageCoudeEconomyUseCase for StubCoudeEconomy {
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64) -> Result<Vec<TauntEvent>, DomainError> { unimplemented!() }
    async fn steal(&self, _: &str, _: &str, _: &str, _: i64) -> Result<manage_economy::StealOutcome, DomainError> { unimplemented!() }
    async fn steal_fail_penalty(&self, _: &str, _: &str, _: i64) -> Result<(i64, Vec<TauntEvent>), DomainError> { unimplemented!() }
    async fn record_casino_win(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_casino_loss(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_casino_faillite(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn count_casino_today(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn sum_casino_gains_today(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn count_steal_today(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
}

pub struct StubCoudeBets;
#[async_trait]
impl ManageCoudeBetsUseCase for StubCoudeBets {
    async fn place(&self, _: NewCoudeBet) -> Result<PlaceBetOutcome, DomainError> { unimplemented!() }
    async fn list_for_combat(&self, _: Uuid) -> Result<Vec<Bet>, DomainError> { unimplemented!() }
    async fn resolve(&self, _: Uuid, _: Option<String>) -> Result<ResolveBetsOutcome, DomainError> { unimplemented!() }
    async fn refund(&self, _: Uuid) -> Result<RefundSummary, DomainError> { unimplemented!() }
}

pub struct StubCoudeCombats;
#[async_trait]
impl manage_combats::ManageCoudeCombatsUseCase for StubCoudeCombats {
    async fn list(&self, _: &str, _: Option<&str>, _: i64) -> Result<Vec<Combat>, DomainError> { unimplemented!() }
    async fn get(&self, _: Uuid) -> Result<Combat, DomainError> { unimplemented!() }
    async fn get_pending_for_attacker(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> { unimplemented!() }
    async fn get_pending_for_defender(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> { unimplemented!() }
    async fn list_expired_pending(&self) -> Result<Vec<Combat>, DomainError> { unimplemented!() }
    async fn get_betting_for_participant(&self, _: &str, _: &str) -> Result<Option<Combat>, DomainError> { unimplemented!() }
    async fn create(&self, _: NewCoudeCombat) -> Result<Combat, DomainError> { unimplemented!() }
    async fn cancel(&self, _: Uuid) -> Result<(), DomainError> { unimplemented!() }
    async fn resolve(&self, _: Uuid, _: CombatResolution) -> Result<(), DomainError> { unimplemented!() }
    async fn set_betting(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { unimplemented!() }
    async fn expire(&self, _: Uuid) -> Result<(), DomainError> { unimplemented!() }
    async fn set_defender_special(&self, _: Uuid, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubCoudePlayers;
#[async_trait]
impl manage_players::ManageCoudePlayersUseCase for StubCoudePlayers {
    async fn get_or_create(&self, _: String, _: String, _: String) -> Result<Player, DomainError> { unimplemented!() }
    async fn get(&self, _: &str, _: &str) -> Result<Player, DomainError> { unimplemented!() }
    async fn list(&self, _: &str) -> Result<Vec<Player>, DomainError> { unimplemented!() }
    async fn random_active(&self, _: &str, _: i64) -> Result<Vec<Player>, DomainError> { unimplemented!() }
    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> { unimplemented!() }
    async fn update_class(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn add_xp(&self, _: &str, _: &str, _: i64) -> Result<XpProgress, DomainError> { unimplemented!() }
    async fn spend_stat_point(&self, _: &str, _: &str, _: CombatStat) -> Result<Player, DomainError> { unimplemented!() }
    async fn reset_stats(&self, _: &str, _: &str, _: i64) -> Result<Player, DomainError> { unimplemented!() }
    async fn record_win(&self, _: &str, _: &str, _: i64, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_loss(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_draw(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn increment_cowardice(&self, _: &str, _: &str) -> Result<i32, DomainError> { unimplemented!() }
    async fn increment_chaos(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn record_coins_earned(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn record_coins_lost(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> { unimplemented!() }
    async fn update_hp(&self, _: &str, _: &str, _: i32, _: i32) -> Result<(), DomainError> { unimplemented!() }
    async fn full_heal(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn regen_hp_tick(&self, _: f64, _: f64, _: f64, _: f64) -> Result<u64, DomainError> { unimplemented!() }
}

pub struct StubResolveBettingBatch;
#[async_trait] impl resolve_betting_batch::ResolveBettingBatchUseCase for StubResolveBettingBatch {
    async fn resolve_batch(&self) -> Result<Vec<resolve_betting_batch::ResolvedBettingCombatOutput>, DomainError> { unimplemented!() }
}
pub struct StubExpireCombatsBatch;
#[async_trait] impl expire_combats_batch::ExpireCombatsBatchUseCase for StubExpireCombatsBatch {
    async fn expire_batch(&self) -> Result<Vec<expire_combats_batch::ExpiredCombatOutput>, DomainError> { unimplemented!() }
}
pub struct StubResolveCombatNow;
#[async_trait] impl resolve_combat_now::ResolveCombatNowUseCase for StubResolveCombatNow {
    async fn resolve_now(&self, _: Uuid) -> Result<resolve_combat_now::ResolveCombatNowOutput, DomainError> { unimplemented!() }
}

pub struct StubSlotUc;
#[async_trait] impl manage_slot::ManageSlotUseCase for StubSlotUc {
    async fn spin(&self, _: manage_slot::SpinCommand) -> Result<manage_slot::SpinResult, DomainError> { unimplemented!() }
    async fn claim_daily_bonus(&self, _: manage_slot::SpinCommand) -> Result<manage_slot::SpinResult, DomainError> { unimplemented!() }
    async fn get_jackpot_pool(&self, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn recent_spins(&self, _: &str, _: i64) -> Result<Vec<SlotSpin>, DomainError> { Ok(vec![]) }
    async fn top_winners(&self, _: &str, _: i64, _: i64) -> Result<Vec<SlotTopWinner>, DomainError> { Ok(vec![]) }
}

pub struct StubWheelUc;
#[async_trait] impl manage_wheel::ManageWheelUseCase for StubWheelUc {
    async fn spin(&self, _: manage_wheel::WheelSpinCommand) -> Result<manage_wheel::WheelSpinResult, DomainError> { unimplemented!() }
    async fn recent_spins(&self, _: &str, _: i64) -> Result<Vec<WheelSpin>, DomainError> { Ok(vec![]) }
    async fn top_winners(&self, _: &str, _: i64, _: i64) -> Result<Vec<WheelTopWinner>, DomainError> { Ok(vec![]) }
}

pub struct StubResolveFriendlyDuel;
#[async_trait] impl ResolveFriendlyDuelUseCase for StubResolveFriendlyDuel {
    async fn resolve(&self, _: FriendlyDuelInput) -> Result<FriendlyDuelOutput, DomainError> { unimplemented!() }
}

pub struct StubPlayToutOuRien;
#[async_trait] impl play_tout_ou_rien::PlayToutOuRienUseCase for StubPlayToutOuRien {
    async fn play(&self, _: play_tout_ou_rien::PlayToutOuRienCommand) -> Result<play_tout_ou_rien::ToutOuRienResolution, DomainError> { unimplemented!() }
}

pub struct StubPlayTravaux;
#[async_trait] impl play_travaux::PlayTravauxUseCase for StubPlayTravaux {
    async fn play(&self, _: play_travaux::PlayTravauxCommand) -> Result<play_travaux::TravauxResolution, DomainError> { unimplemented!() }
}

pub struct StubRollSteal;
#[async_trait] impl roll_steal::RollStealUseCase for StubRollSteal {
    async fn roll(&self, _: roll_steal::RollStealCommand) -> Result<roll_steal::StealRoll, DomainError> {
        Ok(roll_steal::StealRoll { thief_d20: 10, victim_d20: 5, steal_pct_bp: 1200 })
    }
}

pub struct StubCoudeFlavorTemplates;
#[async_trait] impl FlavorTemplatesRepository for StubCoudeFlavorTemplates {
    async fn random_by_key(&self, _: &str, _: &str) -> Result<Option<String>, DomainError> { Ok(None) }
}

pub struct StubCoudeCatalog;
#[async_trait] impl manage_catalog::ManageCoudeCatalogUseCase for StubCoudeCatalog {
    async fn get_catalog(&self) -> Result<manage_catalog::Catalog, DomainError> { unimplemented!() }
}

pub struct StubCoudeCashbox;
#[async_trait] impl manage_cashbox::ManageCoudeCashboxUseCase for StubCoudeCashbox {
    async fn get_cashbox(&self, _: &str) -> Result<Cashbox, DomainError> { unimplemented!() }
    async fn deposit(&self, _: &str, _: i64, _: CashboxSource) -> Result<(), DomainError> { unimplemented!() }
    async fn redistribute_weekly(&self, _: &str) -> Result<Option<manage_cashbox::RedistributionOutcome>, DomainError> { unimplemented!() }
    async fn redistribute_due_guilds(&self, _: i64) -> Result<Vec<(String, manage_cashbox::RedistributionOutcome)>, DomainError> { unimplemented!() }
    async fn list_redistributions(&self, _: &str, _: i64) -> Result<Vec<CashboxRedistribution>, DomainError> { unimplemented!() }
    async fn list_entries(&self, _: uuid::Uuid) -> Result<Vec<CashboxRedistributionEntry>, DomainError> { unimplemented!() }
}

pub struct StubCoudeStealProtections;
#[async_trait] impl manage_steal_protections::ManageCoudeStealProtectionsUseCase for StubCoudeStealProtections {
    async fn list_active(&self, _: &str, _: &str) -> Result<Vec<StealProtection>, DomainError> { unimplemented!() }
    async fn price_for(&self, _: &str, _: StealProtectionDuration) -> Result<i64, DomainError> { unimplemented!() }
    async fn subscribe(&self, _: &str, _: &str, _: &str, _: StealProtectionDuration) -> Result<chrono::DateTime<chrono::Utc>, DomainError> { unimplemented!() }
    async fn try_trigger(&self, _: &str, _: &str) -> Result<Option<StealProtectionTrigger>, DomainError> { unimplemented!() }
}

pub struct StubCoudeStealBoosts;
#[async_trait] impl manage_steal_boosts::ManageCoudeStealBoostsUseCase for StubCoudeStealBoosts {
    async fn list_active(&self, _: &str, _: &str) -> Result<Vec<StealBoost>, DomainError> { unimplemented!() }
    async fn price_for(&self, _: &str, _: StealBoostDuration) -> Result<i64, DomainError> { unimplemented!() }
    async fn subscribe(&self, _: &str, _: &str, _: &str, _: StealBoostDuration) -> Result<chrono::DateTime<chrono::Utc>, DomainError> { unimplemented!() }
    async fn total_bonus(&self, _: &str, _: &str) -> Result<i32, DomainError> { unimplemented!() }
}

pub struct StubCoudeTaunts;
#[async_trait] impl manage_taunts::ManageCoudeTauntsUseCase for StubCoudeTaunts {
    async fn on_player_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_player_lost(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_player_drew(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn on_player_stolen_from(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_player_defended_steal(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn on_bj_natural(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_bj_hand_won(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_bj_hand_bust(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_bankruptcy(&self, _: &str, _: &str) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_jackpot(&self, _: &str, _: &str, _: i64) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn on_generous_donor(&self, _: &str, _: &str, _: i64) -> Result<Option<TauntEvent>, DomainError> { unimplemented!() }
    async fn get_config(&self, _: &str) -> Result<TauntsConfig, DomainError> { unimplemented!() }
    async fn set_channel(&self, _: &str, _: Option<&str>) -> Result<(), DomainError> { unimplemented!() }
    async fn set_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { unimplemented!() }
    async fn set_rename_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { unimplemented!() }
    async fn set_messages_enabled(&self, _: &str, _: bool) -> Result<(), DomainError> { unimplemented!() }
    async fn set_opt_out(&self, _: &str, _: &str, _: bool) -> Result<(), DomainError> { unimplemented!() }
    async fn is_opted_out(&self, _: &str, _: &str) -> Result<bool, DomainError> { unimplemented!() }
    async fn list_opt_outs(&self, _: &str) -> Result<Vec<String>, DomainError> { unimplemented!() }
}

pub struct StubWalletUc;
#[async_trait] impl manage_wallet::ManageWalletUseCase for StubWalletUc {
    async fn credit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<manage_wallet::WalletMutation, DomainError> { unimplemented!() }
    async fn debit(&self, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<manage_wallet::WalletMutation, DomainError> { unimplemented!() }
    async fn transfer(&self, _: &str, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<Vec<TauntEvent>, DomainError> { unimplemented!() }
    async fn get_balance(&self, _: &str, _: &str) -> Result<i64, DomainError> { unimplemented!() }
    async fn credit_tx(&self, _: &mut dyn sentinel_core::ports::uow::DbTx, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<manage_wallet::TxWalletMutation, DomainError> { unimplemented!() }
    async fn debit_tx(&self, _: &mut dyn sentinel_core::ports::uow::DbTx, _: &str, _: &str, _: i64, _: &str, _: &str) -> Result<manage_wallet::TxWalletMutation, DomainError> { unimplemented!() }
    async fn post_commit_taunts(&self, _: &str, _: &str, _: &manage_wallet::TxWalletMutation) -> Vec<TauntEvent> { unimplemented!() }
}

pub struct StubCoudeHeist;
#[async_trait] impl manage_heist::ManageCoudeHeistUseCase for StubCoudeHeist {
    async fn get_cooldown_status(&self, _: &str, _: &str) -> Result<manage_heist::HeistCooldownStatus, DomainError> { unimplemented!() }
    async fn get_prison_status(&self, _: &str, _: &str) -> Result<manage_heist::PrisonStatusInfo, DomainError> { unimplemented!() }
    async fn attempt_heist(&self, _: &str, _: &str) -> Result<HeistOutcome, DomainError> { unimplemented!() }
}

pub struct StubCoudeCurses;
#[async_trait] impl sentinel_api::ports::inbound::coude::manage_curses::ManageCoudeCursesUseCase for StubCoudeCurses {
    async fn cast(&self, _: &str, _: &str, _: &str, _: &str, _: Option<sentinel_core::domain::entities::coude::curse::CurseKind>) -> Result<sentinel_api::ports::inbound::coude::manage_curses::CastedCurse, DomainError> { unimplemented!() }
    async fn get_active(&self, _: &str, _: &str) -> Result<Option<sentinel_core::domain::entities::coude::curse::ActiveCurse>, DomainError> { Ok(None) }
    async fn lift_own(&self, _: &str, _: &str, _: &str) -> Result<sentinel_core::domain::entities::coude::curse::ActiveCurse, DomainError> { unimplemented!() }
}

pub struct StubCoudeSafetyNet;
#[async_trait] impl sentinel_api::ports::inbound::coude::manage_safety_net::ManageCoudeSafetyNetUseCase for StubCoudeSafetyNet {
    async fn try_activate(&self, _: &str, _: &str, _: i64) -> Result<Option<sentinel_core::domain::entities::coude::safety_net::ActiveSafetyNet>, DomainError> { Ok(None) }
    async fn get_active(&self, _: &str, _: &str) -> Result<Option<sentinel_core::domain::entities::coude::safety_net::ActiveSafetyNet>, DomainError> { Ok(None) }
    async fn list_active(&self, _: &str) -> Result<Vec<sentinel_core::domain::entities::coude::safety_net::ActiveSafetyNet>, DomainError> { Ok(vec![]) }
}

pub struct StubCoudeVendetta;
#[async_trait] impl sentinel_api::ports::inbound::coude::manage_vendetta::ManageCoudeVendettaUseCase for StubCoudeVendetta {
    async fn declare(&self, _: &str, _: &str, _: &str) -> Result<uuid::Uuid, DomainError> { Ok(uuid::Uuid::new_v4()) }
    async fn get_active(&self, _: &str, _: &str, _: &str) -> Result<Option<sentinel_core::domain::entities::coude::vendetta::ActiveVendetta>, DomainError> { Ok(None) }
    async fn resolve(&self, _: uuid::Uuid, _: bool) -> Result<(), DomainError> { Ok(()) }
    async fn list_by_challenger(&self, _: &str, _: &str) -> Result<Vec<sentinel_core::domain::entities::coude::vendetta::ActiveVendetta>, DomainError> { Ok(vec![]) }
}

pub struct StubCoudeToutOuRien;
#[async_trait] impl sentinel_api::ports::outbound::coude::tout_ou_rien_repository::ToutOuRienRepository for StubCoudeToutOuRien {
    async fn record(&self, _: &str, _: &str, _: &str, _: i64, _: sentinel_core::domain::entities::coude::tout_ou_rien_log::ToutOuRienLogOutcome, _: i64) -> Result<(), DomainError> { Ok(()) }
    async fn memorial(&self, _: &str, _: i64) -> Result<Vec<sentinel_core::domain::entities::coude::tout_ou_rien_log::ToutOuRienLogEntry>, DomainError> { Ok(vec![]) }
    async fn user_stats(&self, _: &str, _: &str) -> Result<sentinel_core::domain::entities::coude::tout_ou_rien_log::ToutOuRienUserStats, DomainError> { Ok(Default::default()) }
}

pub struct StubCoudeBounty;
#[async_trait] impl sentinel_api::ports::outbound::coude::bounty_repository::BountyRepository for StubCoudeBounty {
    async fn open(&self, _: &str, _: &str, _: i64) -> Result<uuid::Uuid, DomainError> { Ok(uuid::Uuid::new_v4()) }
    async fn get_open(&self, _: &str, _: &str) -> Result<Option<sentinel_core::domain::entities::coude::bounty::ActiveBounty>, DomainError> { Ok(None) }
    async fn contribute(&self, _: uuid::Uuid, _: &str, _: &str, _: i64) -> Result<i64, DomainError> { Ok(0) }
    async fn claim(&self, _: uuid::Uuid, _: &str) -> Result<i64, DomainError> { Ok(0) }
}

pub struct StubCoudeRefusalCount;
#[async_trait] impl sentinel_api::ports::outbound::coude::refusal_count_repository::RefusalCountRepository for StubCoudeRefusalCount {
    async fn increment(&self, _: &str, _: &str, _: &str) -> Result<i32, DomainError> { Ok(0) }
    async fn get(&self, _: &str, _: &str, _: &str) -> Result<Option<sentinel_core::domain::entities::coude::refusal_count::RefusalCount>, DomainError> { Ok(None) }
    async fn reset(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

pub struct StubCoudeUltimate;
#[async_trait] impl sentinel_api::ports::outbound::coude::ultimate_repository::UltimateRepository for StubCoudeUltimate {
    async fn activate(&self, _: &str, _: &str, _: sentinel_core::domain::entities::coude::ultimate::UltimateKind) -> Result<(), DomainError> { Ok(()) }
    async fn get(&self, g: &str, u: &str) -> Result<sentinel_core::domain::entities::coude::ultimate::UltimateState, DomainError> {
        Ok(sentinel_core::domain::entities::coude::ultimate::UltimateState {
            guild_id: g.into(),
            user_id: u.into(),
            pending_kind: None,
            last_used_at: None,
            activated_at: None,
        })
    }
    async fn consume_pending(&self, _: &str, _: &str) -> Result<Option<sentinel_core::domain::entities::coude::ultimate::UltimateKind>, DomainError> { Ok(None) }
}

pub struct StubCoudeCoalition;
#[async_trait] impl sentinel_api::ports::outbound::coude::coalition_repository::CoalitionRepository for StubCoudeCoalition {
    async fn create_with_first_member(&self, _: &str, _: &str, _: &str, _: &str, _: i64) -> Result<uuid::Uuid, DomainError> { Ok(uuid::Uuid::new_v4()) }
    async fn add_member(&self, _: uuid::Uuid, _: &str, _: &str) -> Result<sentinel_core::domain::entities::coude::coalition::ActiveCoalition, DomainError> {
        use chrono::Utc;
        Ok(sentinel_core::domain::entities::coude::coalition::ActiveCoalition {
            id: uuid::Uuid::new_v4(),
            guild_id: String::new(),
            target_id: String::new(),
            opened_at: Utc::now(),
            expires_at: Utc::now(),
            status: sentinel_core::domain::entities::coude::coalition::CoalitionStatus::Forming,
            broken_by: None,
            broken_at: None,
            members: vec![],
        })
    }
    async fn get_active(&self, _: &str, _: &str) -> Result<Option<sentinel_core::domain::entities::coude::coalition::ActiveCoalition>, DomainError> { Ok(None) }
    async fn mark_broken(&self, _: uuid::Uuid, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn is_member_of_active_coalition_against(&self, _: &str, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
}

// ── Stubs pour les nouveaux repos ──

pub struct StubUserActivityRepo;
#[async_trait] impl UserActivityRepository for StubUserActivityRepo {
    async fn create(&self, _: &UserActivity) -> Result<(), DomainError> { Ok(()) }
    async fn list(&self, _: &str, _: &str, _: Option<&str>, _: i64, _: i64) -> Result<Vec<UserActivity>, DomainError> { Ok(vec![]) }
}

pub struct StubWelcomeConfigRepo;
#[async_trait] impl WelcomeConfigRepository for StubWelcomeConfigRepo {
    async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError> {
        Ok(WelcomeConfigData { guild_id: guild_id.into(), welcome_enabled: true, welcome_channel_id: None, welcome_message: String::new(), welcome_embed_color: "3498db".into(), welcome_dm_enabled: false, welcome_dm_message: String::new(), leave_enabled: false, leave_channel_id: None, leave_message: String::new(), rules_enabled: false, rules_channel_id: None, rules_message: String::new(), rules_role_id: None, rules_button_label: String::new(), counter_enabled: false, counter_channel_id: None, counter_format: String::new(), voice_counter_enabled: false, voice_counter_channel_id: None, voice_counter_format: String::new(), anniversary_enabled: false, anniversary_channel_id: None, anniversary_message: String::new(), rejoin_message: String::new(), welcome_title: String::new(), welcome_image_url: String::new(), welcome_footer_text: String::new(), rejoin_title: String::new(), rejoin_image_url: String::new(), rejoin_footer_text: String::new(), leave_title: String::new(), leave_image_url: String::new(), leave_footer_text: String::new(), anniversary_title: String::new(), anniversary_image_url: String::new(), anniversary_footer_text: String::new() })
    }
    async fn save_config(&self, _: &str, d: &WelcomeConfigData) -> Result<WelcomeConfigData, DomainError> { Ok(d.clone()) }
}

pub struct StubAutomodReviewRepo;
#[async_trait] impl sentinel_api::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository for StubAutomodReviewRepo {
    async fn create(&self, _: sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview) -> Result<sentinel_core::domain::entities::moderation::review::automod::AutomodReview, DomainError> { Err(DomainError::Internal("stub".into())) }
    async fn create_or_merge(&self, _: sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview, _: bool) -> Result<(sentinel_core::domain::entities::moderation::review::automod::AutomodReview, bool), DomainError> { Err(DomainError::Internal("stub".into())) }
    async fn get(&self, _: Uuid) -> Result<Option<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>, DomainError> { Ok(None) }
    async fn list_pending(&self, _: &str, _: i64) -> Result<Vec<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>, DomainError> { Ok(vec![]) }
    async fn list_recent(&self, _: &str, _: i64) -> Result<Vec<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>, DomainError> { Ok(vec![]) }
    async fn resolve(&self, _: Uuid, _: &str, _: &str, _: &str, _: &str) -> Result<sentinel_core::domain::entities::moderation::review::automod::AutomodReview, DomainError> { Err(DomainError::Internal("stub".into())) }
}

pub struct StubDiscordActionMessageRepo;
#[async_trait] impl sentinel_api::ports::outbound::audit::discord_action_message_repository::DiscordActionMessageRepository for StubDiscordActionMessageRepo {
    async fn register(&self, _: sentinel_core::domain::entities::audit::discord_action_message::NewDiscordActionMessage) -> Result<(), DomainError> { Ok(()) }
    async fn list_for_action(&self, _: Uuid) -> Result<Vec<sentinel_core::domain::entities::audit::discord_action_message::DiscordActionMessage>, DomainError> { Ok(vec![]) }
    async fn get(&self, _: Uuid, _: &str) -> Result<Option<sentinel_core::domain::entities::audit::discord_action_message::DiscordActionMessage>, DomainError> { Ok(None) }
    async fn touch_edited(&self, _: Uuid, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn delete(&self, _: Uuid, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn find_by_message(&self, _: &str, _: &str, _: &str) -> Result<Option<sentinel_core::domain::entities::audit::discord_action_message::DiscordActionMessage>, DomainError> { Ok(None) }
}

pub struct StubExportUC;
#[async_trait] impl sentinel_api::application::system::export_service::ExecuteExportUseCase for StubExportUC {
    async fn execute(&self, _: &str, _: &str, _: &str, _: i64) -> Result<sentinel_api::application::system::export_service::ExportResult, DomainError> {
        Ok(sentinel_api::application::system::export_service::ExportResult { data: String::new(), row_count: 0 })
    }
}

pub struct StubEvidenceRepo;
#[async_trait] impl EvidenceRepository for StubEvidenceRepo {
    async fn add(&self, _: Uuid, _: &str, _: Option<&str>, _: &str, _: &str) -> Result<EvidenceEntry, DomainError> { Err(DomainError::Internal("stub".into())) }
    async fn list(&self, _: Uuid) -> Result<Vec<EvidenceEntry>, DomainError> { Ok(vec![]) }
}

pub struct StubReviewRepo;
#[async_trait] impl ReviewRepository for StubReviewRepo {
    async fn add(&self, _: Uuid, _: &str, _: &str, _: &str, _: Option<&str>) -> Result<ReviewEntry, DomainError> { Err(DomainError::Internal("stub".into())) }
    async fn list_pending(&self, _: &str) -> Result<Vec<ReviewEntry>, DomainError> { Ok(vec![]) }
    async fn resolve(&self, _: Uuid, _: &str, _: &str, _: Option<&str>, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn get_guild_id(&self, _: Uuid) -> Result<Option<String>, DomainError> { Ok(None) }
}

pub struct StubModstatsRepo;
#[async_trait] impl ModstatsRepository for StubModstatsRepo {
    async fn top_moderators(&self, _: &str, _: i32, _: i64) -> Result<Vec<ModeratorStat>, DomainError> { Ok(vec![]) }
}

pub struct StubGameRepo;
#[async_trait] impl GameRepository for StubGameRepo {
    async fn list(&self, _: &str) -> Result<Vec<Game>, DomainError> { Ok(vec![]) }
    async fn list_by_category(&self, _: &str, _: Option<&str>) -> Result<Vec<Game>, DomainError> { Ok(vec![]) }
    async fn create(&self, _: &str, _: &str, _: &str, _: Option<&str>, _: Option<&str>, _: Option<&str>) -> Result<Game, DomainError> { Err(DomainError::Internal("stub".into())) }
    async fn update(&self, _: &str, _: &str, _: Option<&str>, _: Option<Option<&str>>, _: Option<Option<&str>>) -> Result<Option<Game>, DomainError> { Ok(None) }
    async fn delete(&self, _: &str, _: &str) -> Result<bool, DomainError> { Ok(false) }
    async fn find_by_name(&self, _: &str, _: &str) -> Result<Option<Game>, DomainError> { Ok(None) }
    async fn set_role_id(&self, _: &str, _: &str, _: Option<&str>) -> Result<Option<Game>, DomainError> { Ok(None) }
    async fn save_panel(&self, _: &str, _: &str, _: &str, _: Option<&str>) -> Result<GamePanel, DomainError> { Err(DomainError::Internal("stub".into())) }
    async fn find_panel_by_message(&self, _: &str, _: &str) -> Result<Option<GamePanel>, DomainError> { Ok(None) }
    async fn list_panels(&self, _: &str) -> Result<Vec<GamePanel>, DomainError> { Ok(vec![]) }
}

pub struct StubSponsorshipRepo;
#[async_trait] impl SponsorshipRepository for StubSponsorshipRepo {
    async fn create(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn list(&self, _: &str) -> Result<Vec<Sponsorship>, DomainError> { Ok(vec![]) }
}

pub struct StubTempRoleRepo;
#[async_trait] impl TempRoleRepository for StubTempRoleRepo {
    async fn create(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn list_active(&self, _: &str) -> Result<Vec<TempRole>, DomainError> { Ok(vec![]) }
    async fn delete(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

pub struct StubPendingActionRepo;
#[async_trait] impl PendingActionRepository for StubPendingActionRepo {
    async fn create(&self, _: &str, _: &str, _: &str, _: &str, _: &str, _: &str, _: &str, _: Option<&str>, _: Option<i64>) -> Result<Uuid, DomainError> { Ok(Uuid::new_v4()) }
    async fn list_pending(&self, _: &str) -> Result<Vec<PendingAction>, DomainError> { Ok(vec![]) }
    async fn get_guild_id(&self, _: Uuid) -> Result<Option<String>, DomainError> { Ok(None) }
    async fn resolve(&self, _: Uuid, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
}

pub struct StubBlackjackTableRepo;
#[async_trait] impl BlackjackTableRepository for StubBlackjackTableRepo {
    async fn create(&self, _: &str, _: &str, _: &str, _: &str, _: &serde_json::Value) -> Result<BlackjackTable, DomainError> { Err(DomainError::Internal("stub".into())) }
    async fn get_status_and_guild(&self, _: &str) -> Result<Option<(String, String)>, DomainError> { Ok(None) }
    async fn count_players(&self, _: &str) -> Result<i64, DomainError> { Ok(0) }
    async fn add_player(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn touch_activity(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn list_players(&self, _: &str) -> Result<Vec<BlackjackTablePlayer>, DomainError> { Ok(vec![]) }
    async fn find_open_by_channel(&self, _: &str) -> Result<Option<BlackjackTable>, DomainError> { Ok(None) }
    async fn get_guild_id(&self, _: &str) -> Result<Option<String>, DomainError> { Ok(None) }
    async fn close(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn list_games(&self, _: &str) -> Result<Vec<serde_json::Value>, DomainError> { Ok(vec![]) }
}

// ══════════════════════════════════════════════════════════
// TestAppState builder
// ══════════════════════════════════════════════════════════

/// Construit un AppState de base avec tous les stubs.
fn base_state() -> AppState {
    // On branche sur le compose de test (6380/5433) pour que les branches
    // redis/sqlx direct des handlers (caches, api_user_guilds, modstats, etc.)
    // soient reellement executees pendant les tests d'integration HTTP.
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6380".to_string());
    let redis_client = redis::Client::open(redis_url).unwrap();
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".to_string());
    let pg_pool = sqlx::PgPool::connect_lazy(&db_url).unwrap();

    AppState {
        analyze_uc: Arc::new(StubAnalyzeMessage),
        analyze_image_uc: Arc::new(StubAnalyzeImage),
        rules_uc: Arc::new(StubRules),
        infractions_uc: Arc::new(StubInfractions),
        tickets_uc: Arc::new(StubTickets),
        security_uc: Arc::new(StubSecurity),
        moderation_uc: Arc::new(StubModeration),
        stats_uc: Arc::new(StubStats),
        voice_channels_uc: Arc::new(StubVoiceChannels),
        watched_users_uc: Arc::new(StubWatchedUsers),
        audit_logs_uc: Arc::new(StubAuditLogs),
        levels_uc: Arc::new(StubLevels),
        announcements_uc: Arc::new(StubAnnouncements),
        confessions_uc: Arc::new(StubConfessions),
        role_panels_uc: Arc::new(StubRolePanels),
        notes_uc: Arc::new(StubNotes),
        reminders_uc: Arc::new(StubReminders),
        strikes_uc: Arc::new(StubStrikes),
        analytics_repo: Arc::new(StubAnalyticsRepo),
        daily_activity_repo: Arc::new(StubDailyActivityRepo),
        log_repo: Arc::new(StubLogRepo),
        guild_repo: Arc::new(StubGuildRepo),
        bot_config_repo: Arc::new(StubBotConfigRepo),
        discord_role_repo: Arc::new(StubDiscordRoleRepo),
        members_uc: Arc::new(StubMembers),
        wallet_repo: Arc::new(StubWalletRepo),
        wallet_uc: Arc::new(StubWalletUc),
        blackjack_svc: Arc::new(sentinel_api::application::casino::blackjack_service::BlackjackService::new(
            Arc::new(StubBlackjackRepo),
            Arc::new(StubWalletRepo),
            Arc::new(StubWalletUc),
        )),
        coude_players_uc: Arc::new(StubCoudePlayers),
        coude_combats_uc: Arc::new(StubCoudeCombats),
        coude_bets_uc: Arc::new(StubCoudeBets),
        coude_economy_uc: Arc::new(StubCoudeEconomy),
        coude_inventory_uc: Arc::new(StubCoudeInventory),
        coude_social_uc: Arc::new(StubCoudeSocial),
        coude_catalog_uc: Arc::new(StubCoudeCatalog),
        coude_cashbox_uc: Arc::new(StubCoudeCashbox),
        coude_steal_protections_uc: Arc::new(StubCoudeStealProtections),
        coude_steal_boosts_uc: Arc::new(StubCoudeStealBoosts),
        coude_taunts_uc: Arc::new(StubCoudeTaunts),
        coude_heist_uc: Arc::new(StubCoudeHeist),
        coude_curses_uc: Arc::new(StubCoudeCurses),
        coude_safety_net_uc: Arc::new(StubCoudeSafetyNet),
        coude_vendetta_uc: Arc::new(StubCoudeVendetta),
        coude_tout_ou_rien_repo: Arc::new(StubCoudeToutOuRien),
        coude_bounty_repo: Arc::new(StubCoudeBounty),
        coude_refusal_count_repo: Arc::new(StubCoudeRefusalCount),
        coude_coalition_repo: Arc::new(StubCoudeCoalition),
        coude_ultimate_repo: Arc::new(StubCoudeUltimate),
        resolve_betting_batch_uc: Arc::new(StubResolveBettingBatch),
        expire_combats_batch_uc: Arc::new(StubExpireCombatsBatch),
        resolve_combat_now_uc: Arc::new(StubResolveCombatNow),
        slot_uc: Arc::new(StubSlotUc),
        wheel_uc: Arc::new(StubWheelUc),
        resolve_friendly_duel_uc: Arc::new(StubResolveFriendlyDuel),
        play_tout_ou_rien_uc: Arc::new(StubPlayToutOuRien),
        play_travaux_uc: Arc::new(StubPlayTravaux),
        roll_steal_uc: Arc::new(StubRollSteal),
        coude_flavor_templates_repo: Arc::new(StubCoudeFlavorTemplates),
        user_activity_repo: Arc::new(StubUserActivityRepo),
        welcome_config_uc: Arc::new(
            sentinel_api::application::community::manage_welcome_config_service::ManageWelcomeConfigService::new(
                Arc::new(StubWelcomeConfigRepo),
            ),
        ),
        automod_reviews_uc: Arc::new(
            sentinel_api::application::moderation::manage_automod_reviews_service::ManageAutomodReviewsService::new(
                Arc::new(StubAutomodReviewRepo),
            ),
        ),
        discord_action_messages_uc: Arc::new(
            sentinel_api::application::audit::manage_discord_action_messages_service::ManageDiscordActionMessagesService::new(
                Arc::new(StubDiscordActionMessageRepo),
            ),
        ),
        export_uc: Arc::new(StubExportUC),
        evidence_repo: Arc::new(StubEvidenceRepo),
        review_repo: Arc::new(StubReviewRepo),
        modstats_repo: Arc::new(StubModstatsRepo),
        game_repo: Arc::new(StubGameRepo),
        sponsorship_repo: Arc::new(StubSponsorshipRepo),
        temp_role_repo: Arc::new(StubTempRoleRepo),
        pending_action_repo: Arc::new(StubPendingActionRepo),
        blackjack_table_repo: Arc::new(StubBlackjackTableRepo),
        broadcaster: Arc::new(EventBroadcaster::new()),
        job_client: JobClient::new(redis_client.clone(), "test:jobs".into()),
        discord_api: Arc::new(DiscordApiService::new(String::new())),
        inference: Arc::new(sentinel_api::adapters::outbound::inference_service::InferenceService::new(None, None)),
        api_key: String::new(),
        discord_bot_token: String::new(),
        pg_pool,
        redis_client,
        cache: None,
        superadmin_user_ids: Arc::new(Vec::new()),
        discord_oauth_client_id: String::new(),
        discord_oauth_client_secret: String::new(),
        discord_oauth_redirect_uri: String::new(),
        web_front_url: String::new(),
        container_monitor: None,
        rate_limiter: None,
    }
}

/// Construit un AppState avec un mock voice channels injecte.
#[allow(dead_code)]
pub fn build_test_state(voice_uc: Arc<dyn ManageVoiceChannelsUseCase>) -> AppState {
    let mut state = base_state();
    state.voice_channels_uc = voice_uc;
    state
}

/// Construit un AppState avec un mock tickets injecte.
#[allow(dead_code)]
pub fn build_test_state_tickets(tickets_uc: Arc<dyn ManageTicketsUseCase>) -> AppState {
    let mut state = base_state();
    state.tickets_uc = tickets_uc;
    state
}

/// Construit un AppState avec un mock strikes injecte.
#[allow(dead_code)]
pub fn build_test_state_strikes(strikes_uc: Arc<dyn ManageStrikesUseCase>) -> AppState {
    let mut state = base_state();
    state.strikes_uc = strikes_uc;
    state
}

/// Construit un AppState avec un mock rules injecte.
#[allow(dead_code)]
pub fn build_test_state_rules(rules_uc: Arc<dyn ManageRulesUseCase>) -> AppState {
    let mut state = base_state();
    state.rules_uc = rules_uc;
    state
}

/// Construit un AppState avec un mock infractions injecte.
#[allow(dead_code)]
pub fn build_test_state_infractions(infractions_uc: Arc<dyn ManageInfractionsUseCase>) -> AppState {
    let mut state = base_state();
    state.infractions_uc = infractions_uc;
    state
}

/// Construit un AppState avec un mock audit logs injecte.
#[allow(dead_code)]
pub fn build_test_state_audit_logs(audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>) -> AppState {
    let mut state = base_state();
    state.audit_logs_uc = audit_logs_uc;
    state
}

/// Construit un AppState avec un mock watched users injecte.
#[allow(dead_code)]
pub fn build_test_state_watched_users(watched_users_uc: Arc<dyn ManageWatchedUsersUseCase>) -> AppState {
    let mut state = base_state();
    state.watched_users_uc = watched_users_uc;
    state
}

/// Construit un AppState avec un mock user activity repository injecte.
#[allow(dead_code)]
pub fn build_test_state_user_activity(user_activity_repo: Arc<dyn UserActivityRepository>) -> AppState {
    let mut state = base_state();
    state.user_activity_repo = user_activity_repo;
    state
}

/// Construit un AppState avec un mock analyze (text) use case injecte.
#[allow(dead_code)]
pub fn build_test_state_analyze(analyze_uc: Arc<dyn AnalyzeMessageUseCase>) -> AppState {
    let mut state = base_state();
    state.analyze_uc = analyze_uc;
    state
}

/// Construit un AppState avec un mock security use case injecte.
#[allow(dead_code)]
pub fn build_test_state_security(security_uc: Arc<dyn ManageSecurityUseCase>) -> AppState {
    let mut state = base_state();
    state.security_uc = security_uc;
    state
}

/// Construit un AppState avec un mock levels use case injecte.
#[allow(dead_code)]
pub fn build_test_state_levels(levels_uc: Arc<dyn ManageLevelsUseCase>) -> AppState {
    let mut state = base_state();
    state.levels_uc = levels_uc;
    state
}

/// Construit un AppState avec un mock stats use case injecte.
#[allow(dead_code)]
pub fn build_test_state_stats(stats_uc: Arc<dyn ManageStatsUseCase>) -> AppState {
    let mut state = base_state();
    state.stats_uc = stats_uc;
    state
}

/// Construit un AppState avec un mock log repository injecte.
#[allow(dead_code)]
pub fn build_test_state_logs(log_repo: Arc<dyn LogRepository>) -> AppState {
    let mut state = base_state();
    state.log_repo = log_repo;
    state
}

/// Construit un AppState avec un mock guild repository injecte.
#[allow(dead_code)]
pub fn build_test_state_guilds(guild_repo: Arc<dyn GuildRepository>) -> AppState {
    let mut state = base_state();
    state.guild_repo = guild_repo;
    state
}

/// Construit un AppState avec un mock daily activity repository injecte.
#[allow(dead_code)]
pub fn build_test_state_daily_activity(daily_activity_repo: Arc<dyn DailyActivityRepository>) -> AppState {
    let mut state = base_state();
    state.daily_activity_repo = daily_activity_repo;
    state
}

/// Construit un AppState avec un mock analytics repository injecte.
#[allow(dead_code)]
pub fn build_test_state_analytics(analytics_repo: Arc<dyn AnalyticsRepository>) -> AppState {
    let mut state = base_state();
    state.analytics_repo = analytics_repo;
    state
}

/// Construit un AppState avec un mock role panels use case injecte.
#[allow(dead_code)]
pub fn build_test_state_role_panels(role_panels_uc: Arc<dyn ManageRolePanelsUseCase>) -> AppState {
    let mut state = base_state();
    state.role_panels_uc = role_panels_uc;
    state
}

/// Construit un AppState avec un mock welcome config repository injecte.
/// Le repo est wrappe dans le service applicatif pour exposer le use case
/// (l'AppState n'expose plus le repo directement).
#[allow(dead_code)]
pub fn build_test_state_welcome(welcome_config_repo: Arc<dyn WelcomeConfigRepository>) -> AppState {
    let mut state = base_state();
    state.welcome_config_uc = Arc::new(
        sentinel_api::application::community::manage_welcome_config_service::ManageWelcomeConfigService::new(
            welcome_config_repo,
        ),
    );
    state
}

/// Construit un AppState avec un mock bot_config repository injecte.
#[allow(dead_code)]
pub fn build_test_state_bot_config(bot_config_repo: Arc<dyn BotConfigRepository>) -> AppState {
    let mut state = base_state();
    state.bot_config_repo = bot_config_repo;
    state
}

/// Construit un AppState avec un mock wallet repository injecte.
#[allow(dead_code)]
pub fn build_test_state_wallet(wallet_repo: Arc<dyn WalletRepository>) -> AppState {
    let mut state = base_state();
    state.wallet_repo = wallet_repo;
    state
}

/// Construit un AppState avec un mock game repository + MockDiscordApi injectes
/// (create_game appelle discord_api.create_role + edit_role en plus du repo).
#[allow(dead_code)]
pub fn build_test_state_game(game_repo: Arc<dyn GameRepository>) -> AppState {
    let mut state = base_state();
    state.game_repo = game_repo;
    state.discord_api = Arc::new(MockDiscordApi::new());
    state
}

/// Construit un AppState avec un mock DiscordApi injecte.
#[allow(dead_code)]
pub fn build_test_state_discord_api(discord_api: Arc<dyn DiscordApi>) -> AppState {
    let mut state = base_state();
    state.discord_api = discord_api;
    state
}

/// Construit une requete axum avec un RoleContext pre-injecte dans les
/// extensions. Permet de couvrir les branches `rbac.is_some()` des handlers
/// sans passer par le middleware rbac_middleware (qui requerrait un token
/// Discord valide + Redis + api_users seede).
///
/// axum preserve les extensions du Request a travers les middlewares, donc
/// un Extension<RoleContext> pose ici sera vu par les handlers via
/// `Option<Extension<RoleContext>>`.
#[allow(dead_code)]
pub fn request_with_rbac(
    method: &str,
    uri: &str,
    user_id: &str,
    role: Option<sentinel_core::domain::enums::system::role::Role>,
    guild_id: Option<String>,
    body: Option<serde_json::Value>,
) -> axum::http::Request<axum::body::Body> {
    use sentinel_api::adapters::inbound::http::middleware::rbac::RoleContext;
    let mut builder = axum::http::Request::builder().method(method).uri(uri);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let body = match body {
        Some(v) => axum::body::Body::from(serde_json::to_string(&v).unwrap()),
        None => axum::body::Body::empty(),
    };
    let mut req = builder.body(body).unwrap();
    req.extensions_mut().insert(RoleContext {
        discord_user_id: user_id.to_string(),
        role,
        guild_id,
    });
    req
}

// ══════════════════════════════════════════════════════════
// Mock DiscordApi — retourne Ok(()) par defaut pour tous les appels.
// Utilise par les tests qui veulent couvrir le code APRES discord_api
// (log_action + broadcast dans execute_ban/mute/unban, etc.).
// ══════════════════════════════════════════════════════════

#[derive(Default)]
pub struct MockDiscordApi {
    pub calls: std::sync::Mutex<Vec<String>>,
}

impl MockDiscordApi {
    #[allow(dead_code)]
    pub fn new() -> Self { Self::default() }
    #[allow(dead_code)]
    fn record(&self, call: &str) {
        self.calls.lock().unwrap().push(call.into());
    }
}

#[async_trait]
impl DiscordApi for MockDiscordApi {
    async fn list_text_channels(&self, _: &str) -> Result<Vec<DiscordChannel>, DomainError> {
        self.record("list_text_channels");
        Ok(vec![])
    }
    async fn upload_emoji(&self, _: &str, _: &str, _: &[u8], _: &str) -> Result<(String, String, bool), DomainError> {
        self.record("upload_emoji");
        Ok(("emoji_id".into(), "emoji_name".into(), false))
    }
    async fn ban_user(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("ban_user");
        Ok(())
    }
    async fn list_members(&self, _: &str, _: u32) -> Result<Vec<DiscordMember>, DomainError> {
        self.record("list_members");
        Ok(vec![])
    }
    async fn send_dm(&self, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("send_dm");
        Ok(())
    }
    async fn create_role(&self, _: &str, _: &str, _: u32, _: Option<&str>) -> Result<serde_json::Value, DomainError> {
        self.record("create_role");
        Ok(serde_json::json!({"id": "r1", "name": "role"}))
    }
    async fn edit_role(&self, _: &str, _: &str, _: Option<&str>, _: Option<u32>, _: Option<&str>, _: Option<bool>, _: Option<bool>) -> Result<serde_json::Value, DomainError> {
        self.record("edit_role");
        Ok(serde_json::json!({"id": "r1"}))
    }
    async fn delete_role(&self, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("delete_role");
        Ok(())
    }
    async fn unban_user(&self, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("unban_user");
        Ok(())
    }
    async fn remove_timeout(&self, _: &str, _: &str) -> Result<(), DomainError> {
        self.record("remove_timeout");
        Ok(())
    }
    async fn apply_timeout(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> {
        self.record("apply_timeout");
        Ok(())
    }
    async fn get_user_guilds(&self, _: &str) -> Result<Vec<UserGuild>, DomainError> {
        self.record("get_user_guilds");
        Ok(vec![])
    }
    async fn get_user_me(&self, _: &str) -> Result<DiscordUser, DomainError> {
        self.record("get_user_me");
        Ok(DiscordUser { id: "u1".into(), username: "mock".into(), avatar: None })
    }
}

// ── Stub Voice Channels (needed for base_state) ──

pub struct StubVoiceChannels;
#[async_trait]
impl ManageVoiceChannelsUseCase for StubVoiceChannels {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError> { unimplemented!() }
    async fn list_channels(&self, _: &str) -> Result<Vec<VoiceChannel>, DomainError> { unimplemented!() }
    async fn list_history_channels(&self, _: &str, _: i64) -> Result<Vec<VoiceChannel>, DomainError> { unimplemented!() }
    async fn get_voice_config(&self, _: &str) -> Result<VoiceChannelConfig, DomainError> { Ok(VoiceChannelConfig::default()) }
    async fn get_channel_detail(&self, _: &str) -> Result<VoiceChannelDetail, DomainError> { unimplemented!() }
    async fn create_channel(&self, _: CreateVoiceChannelCommand) -> Result<VoiceChannel, DomainError> { unimplemented!() }
    async fn close_channel(&self, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn delete_channel(&self, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn update_channel(&self, _: UpdateVoiceChannelCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn transfer_ownership(&self, _: TransferOwnershipCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn add_co_admin(&self, _: ManageCoAdminCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn remove_co_admin(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn get_whitelist(&self, _: &str, _: &str) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> { unimplemented!() }
    async fn add_to_whitelist(&self, _: ManageWhitelistCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn remove_from_whitelist(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn ban_from_channel(&self, _: BanFromChannelCommand) -> Result<(), DomainError> { unimplemented!() }
    async fn unban_from_channel(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn is_banned(&self, _: &str, _: &str) -> Result<bool, DomainError> { unimplemented!() }
    async fn create_invite_link(&self, _: CreateInviteLinkCommand) -> Result<VoiceChannelInviteLink, DomainError> { unimplemented!() }
    async fn list_invite_links(&self, _: &str) -> Result<Vec<VoiceChannelInviteLink>, DomainError> { unimplemented!() }
    async fn use_invite_link(&self, _: UseInviteLinkCommand) -> Result<VoiceChannelInviteLink, DomainError> { unimplemented!() }
    async fn revoke_invite_link(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn list_themes(&self, _: &str) -> Result<Vec<VoiceChannelTheme>, DomainError> { unimplemented!() }
    async fn create_theme(&self, _: CreateThemeCommand) -> Result<VoiceChannelTheme, DomainError> { unimplemented!() }
    async fn update_theme(&self, _: &str, _: CreateThemeCommand) -> Result<VoiceChannelTheme, DomainError> { unimplemented!() }
    async fn delete_theme(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

//! Test helpers : construit un AppState complet avec des stubs pour tous les traits.
//! Seul le use case sous test est fonctionnel, les autres panic si appeles.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_api::adapters::outbound::ws::broadcaster::EventBroadcaster;
use sentinel_api::adapters::outbound::discord_api::DiscordApi;
use sentinel_api::adapters::outbound::discord_api::DiscordApiService;
use sentinel_api::adapters::outbound::discord_api::DiscordChannel;
use sentinel_api::adapters::outbound::discord_api::DiscordMember;
use sentinel_api::adapters::outbound::discord_api::DiscordUser;
use sentinel_api::adapters::outbound::discord_api::UserGuild;
use sentinel_api::adapters::outbound::job_client::JobClient;
use sentinel_api::ports::inbound::ai::analyze_image::*;
use sentinel_api::ports::inbound::ai::analyze_message::*;
use sentinel_api::ports::inbound::audit::manage_audit_logs::*;
use sentinel_api::ports::inbound::audit::manage_security::*;
use sentinel_api::ports::inbound::audit::manage_stats::*;
use sentinel_api::ports::inbound::audit::manage_watched_users::*;
use sentinel_api::ports::inbound::audit::*;
use sentinel_api::ports::inbound::community::manage_levels::*;
use sentinel_api::ports::inbound::community::manage_members::*;
use sentinel_api::ports::inbound::community::manage_role_panels::*;
use sentinel_api::ports::inbound::community::manage_voice_channels::*;
use sentinel_api::ports::inbound::community::*;
use sentinel_api::ports::inbound::moderation::manage_infractions::*;
use sentinel_api::ports::inbound::moderation::manage_moderation::*;
use sentinel_api::ports::inbound::moderation::manage_notes::*;
use sentinel_api::ports::inbound::moderation::manage_reminders::*;
use sentinel_api::ports::inbound::moderation::manage_rules::*;
use sentinel_api::ports::inbound::moderation::manage_strikes::*;
use sentinel_api::ports::inbound::moderation::*;
use sentinel_api::ports::inbound::system::manage_tickets::*;
use sentinel_api::ports::outbound::audit::analytics_repository::*;
use sentinel_api::ports::outbound::audit::modstats_repository::*;
use sentinel_api::ports::outbound::audit::user_activity_repository::*;
use sentinel_api::ports::outbound::casino::game_repository::*;
use sentinel_api::ports::outbound::community::daily_activity_repository::*;
use sentinel_api::ports::outbound::community::discord_role_repository::*;
use sentinel_api::ports::outbound::community::sponsorship_repository::*;
use sentinel_api::ports::outbound::community::temp_role_repository::*;
use sentinel_api::ports::outbound::community::welcome_config_repository::*;
use sentinel_api::ports::outbound::moderation::evidence_repository::*;
use sentinel_api::ports::outbound::moderation::pending_action_repository::*;
use sentinel_api::ports::outbound::moderation::review_repository::*;
use sentinel_api::ports::outbound::system::bot_config_repository::*;
use sentinel_api::ports::outbound::system::guild_repository::*;
use sentinel_api::ports::outbound::system::log_repository::*;
use sentinel_core::domain::entities::ai::image_analysis::*;
use sentinel_core::domain::entities::ai::message_analysis::*;
use sentinel_core::domain::entities::audit::audit_log::*;
use sentinel_core::domain::entities::audit::dashboard_stats::*;
use sentinel_core::domain::entities::audit::security_event::*;
use sentinel_core::domain::entities::audit::user_activity::*;
use sentinel_core::domain::entities::audit::user_stats::*;
use sentinel_core::domain::entities::audit::watched_user::*;
use sentinel_core::domain::entities::community::daily_activity::*;
use sentinel_core::domain::entities::community::guild_member::*;
use sentinel_core::domain::entities::community::level::*;
use sentinel_core::domain::entities::community::role_panel::*;
use sentinel_core::domain::entities::community::voice_channel::*;
use sentinel_core::domain::entities::moderation::action::applied::*;
use sentinel_core::domain::entities::moderation::action::sanction_reminder::*;
use sentinel_core::domain::entities::moderation::action::strikes::*;
use sentinel_core::domain::entities::moderation::infraction::*;
use sentinel_core::domain::entities::moderation::user_note::*;
use sentinel_core::domain::entities::system::analytics::*;
use sentinel_core::domain::entities::system::bot_config::*;
use sentinel_core::domain::entities::system::discord_role::*;
use sentinel_core::domain::entities::system::guild::*;
use sentinel_core::domain::entities::system::log_entry::*;
use sentinel_core::domain::entities::system::rule::*;
use sentinel_core::domain::entities::system::ticket::*;
use sentinel_core::domain::errors::DomainError;

// Chaque fichier de test d'intégration est compilé comme une crate séparée.
// Du coup Rust voit les helpers comme "unused" dans les tests qui n'en
// consomment qu'une partie — d'où les `#[allow(dead_code)]` ciblés plus bas.

// ══════════════════════════════════════════════════════════
// Stub Use Cases (inbound)
// ══════════════════════════════════════════════════════════

pub struct StubAnalyzeMessage;
#[async_trait]
impl AnalyzeMessageUseCase for StubAnalyzeMessage {
    async fn analyze(&self, _: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
        unimplemented!()
    }
    async fn evaluate_flood(&self, _: &str, _: i32) -> Result<FloodDecision, DomainError> {
        unimplemented!()
    }
    async fn evaluate_attachments(
        &self,
        _: &str,
        _: Vec<String>,
    ) -> Result<sentinel_core::ports::inbound::ai::analyze_message::AttachmentDecision, DomainError>
    {
        unimplemented!()
    }
    async fn evaluate_caps(
        &self,
        _: &str,
    ) -> Result<sentinel_core::ports::inbound::ai::analyze_message::CapsDecision, DomainError> {
        unimplemented!()
    }
}

pub struct StubAnalyzeImage;
#[async_trait]
impl AnalyzeImageUseCase for StubAnalyzeImage {
    async fn analyze_image(&self, _: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> {
        unimplemented!()
    }
}

pub struct StubRules;
#[async_trait]
impl ManageRulesUseCase for StubRules {
    async fn get_rules(&self, _: &str) -> Result<Vec<Rule>, DomainError> {
        unimplemented!()
    }
    async fn get_all_rules(&self) -> Result<Vec<Rule>, DomainError> {
        unimplemented!()
    }
    async fn toggle_rule(&self, _: Uuid, _: bool) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn create_or_update_rule(&self, _: CreateRuleCommand) -> Result<Rule, DomainError> {
        unimplemented!()
    }
    async fn delete_rule(&self, _: &str, _: Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn seed_default_rules(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubInfractions;
#[async_trait]
impl ManageInfractionsUseCase for StubInfractions {
    async fn list_infractions(
        &self,
        _: &str,
        _: InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        unimplemented!()
    }
    async fn list_all_infractions(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> {
        unimplemented!()
    }
    async fn count_today(&self) -> Result<u64, DomainError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> {
        unimplemented!()
    }
    async fn delete_infraction(&self, _: &str) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

pub struct StubTickets;
#[async_trait]
impl ManageTicketsUseCase for StubTickets {
    async fn list_tickets(
        &self,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
        _: i64,
        _: i64,
    ) -> Result<Vec<Ticket>, DomainError> {
        unimplemented!()
    }
    async fn get_ticket_detail(&self, _: &str) -> Result<TicketDetail, DomainError> {
        unimplemented!()
    }
    async fn create_ticket(&self, _: CreateTicketCommand) -> Result<Ticket, DomainError> {
        unimplemented!()
    }
    async fn reply_ticket(&self, _: ReplyTicketCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn close_ticket(&self, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn assign_ticket(&self, _: AssignTicketCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_status(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_ticket_channel(
        &self,
        _: UpdateTicketChannelCommand,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_priority(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_sla(
        &self,
        _: Uuid,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<i32>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn moderated_guilds(
        &self,
        _: &str,
    ) -> Result<std::collections::HashSet<String>, DomainError> {
        Ok(std::collections::HashSet::new())
    }
    async fn bulk_delete_tickets(
        &self,
        _: Option<&str>,
        _: Option<chrono::DateTime<chrono::Utc>>,
        _: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<u64, DomainError> {
        Ok(0)
    }
}

pub struct StubSecurity;
#[async_trait]
impl ManageSecurityUseCase for StubSecurity {
    async fn report_event(
        &self,
        _: ReportSecurityEventCommand,
    ) -> Result<SecurityEvent, DomainError> {
        unimplemented!()
    }
    async fn purge_events(&self, _: &str) -> Result<(u64, u64), DomainError> {
        Ok((0, 0))
    }
    async fn list_events(&self, _: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> {
        unimplemented!()
    }
    async fn analyze_new_member(
        &self,
        _: AnalyzeNewMemberCommand,
    ) -> Result<SecurityDecision, DomainError> {
        unimplemented!()
    }
}

pub struct StubModeration;
#[async_trait]
impl ManageModerationUseCase for StubModeration {
    async fn list_actions(
        &self,
        _: Option<&str>,
        _: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        unimplemented!()
    }
    async fn log_action(&self, _: LogModerationCommand) -> Result<ModerationAction, DomainError> {
        unimplemented!()
    }
    async fn get_history(&self, _: &str, _: &str) -> Result<UserModerationHistory, DomainError> {
        unimplemented!()
    }
    async fn list_bans(
        &self,
        _: Option<&str>,
        _: i64,
        _: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        unimplemented!()
    }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_action(&self, _: Uuid) -> Result<bool, DomainError> {
        unimplemented!()
    }
}

pub struct StubStats;
#[async_trait]
impl ManageStatsUseCase for StubStats {
    async fn record_messages(
        &self,
        _: manage_stats::RecordMessagesCommand,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn record_voice(&self, _: manage_stats::RecordVoiceCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get_user_stats(&self, _: &str, _: &str) -> Result<Option<UserStats>, DomainError> {
        unimplemented!()
    }
    async fn get_guild_overview(&self, _: &str) -> Result<GuildStatsOverview, DomainError> {
        unimplemented!()
    }
    async fn get_leaderboard(&self, _: &str, _: u32) -> Result<Vec<UserStats>, DomainError> {
        unimplemented!()
    }
    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> {
        unimplemented!()
    }
    async fn get_guild_voice_stats(
        &self,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<GuildVoiceStats, DomainError> {
        unimplemented!()
    }
}

pub struct StubWatchedUsers;
#[async_trait]
impl ManageWatchedUsersUseCase for StubWatchedUsers {
    async fn list_watched_users(
        &self,
        _: Option<&str>,
        _: i64,
        _: i64,
    ) -> Result<Vec<WatchedUser>, DomainError> {
        unimplemented!()
    }
    async fn get_user_dossier(
        &self,
        _: &str,
        _: &str,
    ) -> Result<manage_watched_users::UserDossier, DomainError> {
        unimplemented!()
    }
    async fn add_manual_watch(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn remove_manual_watch(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

pub struct StubAuditLogs;
#[async_trait]
impl ManageAuditLogsUseCase for StubAuditLogs {
    async fn create(
        &self,
        cmd: manage_audit_logs::CreateAuditLogCommand,
    ) -> Result<AuditLog, DomainError> {
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
    async fn list(
        &self,
        _: Option<&str>,
        _: manage_audit_logs::AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError> {
        unimplemented!()
    }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

pub struct StubAuditEventCounter;
#[async_trait]
impl sentinel_core::ports::outbound::audit::audit_event_counter::AuditEventCounter
    for StubAuditEventCounter
{
    async fn count_by_event_type(
        &self,
        _guild_id: &str,
        _days: u32,
    ) -> Result<Vec<(String, u64)>, DomainError> {
        Ok(Vec::new())
    }
}

pub struct StubSnapshots;
#[async_trait]
impl sentinel_core::ports::inbound::audit::manage_snapshots::ManageSnapshotsUseCase
    for StubSnapshots
{
    async fn snapshot_daily_all(
        &self,
    ) -> Result<sentinel_core::domain::entities::audit::snapshot::JobReport, DomainError> {
        Ok(sentinel_core::domain::entities::audit::snapshot::JobReport::ok(0, 0))
    }
    async fn snapshot_hourly_all(
        &self,
    ) -> Result<sentinel_core::domain::entities::audit::snapshot::JobReport, DomainError> {
        Ok(sentinel_core::domain::entities::audit::snapshot::JobReport::ok(0, 0))
    }
    async fn retention_cleanup_all(
        &self,
    ) -> Result<sentinel_core::domain::entities::audit::snapshot::JobReport, DomainError> {
        Ok(sentinel_core::domain::entities::audit::snapshot::JobReport::ok(0, 0))
    }
    async fn plan_top_publications(
        &self,
    ) -> Result<sentinel_core::domain::entities::audit::snapshot::TopPublishPlan, DomainError> {
        Ok(
            sentinel_core::domain::entities::audit::snapshot::TopPublishPlan {
                publications: Vec::new(),
                skipped: 0,
            },
        )
    }
    async fn mark_top_published(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubLevels;
#[async_trait]
impl ManageLevelsUseCase for StubLevels {
    async fn record_text_activity(
        &self,
        _: manage_levels::RecordTextActivityCommand,
    ) -> Result<manage_levels::RecordActivityResult, DomainError> {
        unimplemented!()
    }
    async fn record_voice_activity(
        &self,
        _: manage_levels::RecordVoiceActivityCommand,
    ) -> Result<manage_levels::RecordActivityResult, DomainError> {
        unimplemented!()
    }
    async fn add_xp(
        &self,
        _: manage_levels::AddXpCommand,
    ) -> Result<manage_levels::AddXpResult, DomainError> {
        unimplemented!()
    }
    async fn get_user_level(&self, _: &str, _: &str) -> Result<UserLevel, DomainError> {
        unimplemented!()
    }
    async fn get_leaderboard(&self, _: &str, _: i64) -> Result<Vec<UserLevel>, DomainError> {
        unimplemented!()
    }
    async fn get_leaderboard_by_source(
        &self,
        _: &str,
        _: XpSource,
        _: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        unimplemented!()
    }
    async fn set_user_xp(
        &self,
        _: manage_levels::SetUserXpCommand,
    ) -> Result<UserLevel, DomainError> {
        unimplemented!()
    }
    async fn reset_user_xp(
        &self,
        _: &str,
        _: &str,
        _: manage_levels::ResetTarget,
    ) -> Result<UserLevel, DomainError> {
        unimplemented!()
    }
}

#[allow(dead_code)]
pub struct StubAnnouncements;
#[async_trait]
impl sentinel_api::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase
    for StubAnnouncements
{
    async fn create(
        &self,
        _: sentinel_api::ports::inbound::community::manage_announcements::CreateAnnouncementCommand,
    ) -> Result<
        sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement,
        DomainError,
    > {
        unimplemented!()
    }
    async fn update(
        &self,
        _: sentinel_api::ports::inbound::community::manage_announcements::UpdateAnnouncementCommand,
    ) -> Result<
        sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement,
        DomainError,
    > {
        unimplemented!()
    }
    async fn delete(&self, _: uuid::Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get(
        &self,
        _: uuid::Uuid,
    ) -> Result<
        sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement,
        DomainError,
    > {
        unimplemented!()
    }
    async fn list_by_guild(
        &self,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::announcement::ScheduledAnnouncement>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn toggle(&self, _: uuid::Uuid, _: bool) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn fetch_due_and_prepare(
        &self,
        _: chrono::DateTime<chrono::Utc>,
        _: i64,
    ) -> Result<
        Vec<sentinel_api::ports::inbound::community::manage_announcements::RenderedAnnouncement>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn record_run_result(
        &self,
        _: uuid::Uuid,
        _: Vec<sentinel_core::domain::entities::community::announcement::ChannelPostResult>,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn preview(
        &self,
        _: uuid::Uuid,
    ) -> Result<
        sentinel_api::ports::inbound::community::manage_announcements::RenderedAnnouncement,
        DomainError,
    > {
        unimplemented!()
    }
    async fn list_runs(
        &self,
        _: uuid::Uuid,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::announcement::AnnouncementRun>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn record_button_interaction(
        &self,
        _: uuid::Uuid,
        _: Option<uuid::Uuid>,
        _: String,
        _: Option<String>,
        _: String,
        _: Option<String>,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn list_button_interactions(
        &self,
        _: uuid::Uuid,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::announcement::ButtonInteraction>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn retention_cleanup_all(
        &self,
    ) -> Result<
        sentinel_core::ports::inbound::community::manage_announcements::RetentionCleanupSummary,
        DomainError,
    > {
        Ok(
            sentinel_core::ports::inbound::community::manage_announcements::RetentionCleanupSummary {
                guilds_processed: 0,
                guilds_skipped: 0,
                rows_deleted: 0,
            },
        )
    }
}

#[allow(dead_code)]
pub struct StubConfessions;
#[async_trait]
impl sentinel_api::ports::inbound::community::manage_confessions::ManageConfessionsUseCase
    for StubConfessions
{
    async fn create(
        &self,
        _: sentinel_api::ports::inbound::community::manage_confessions::CreateConfessionCommand,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn update_message_refs(
        &self,
        _: uuid::Uuid,
        _: String,
        _: String,
        _: Option<String>,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn edit_content(
        &self,
        _: uuid::Uuid,
        _: &str,
        _: String,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn delete(
        &self,
        _: uuid::Uuid,
        _: String,
        _: Option<String>,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn get(
        &self,
        _: uuid::Uuid,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn get_by_message_id(
        &self,
        _: &str,
    ) -> Result<
        Option<sentinel_core::domain::entities::community::confession::Confession>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn get_by_public_number(
        &self,
        _: &str,
        _: i32,
    ) -> Result<sentinel_core::domain::entities::community::confession::Confession, DomainError>
    {
        unimplemented!()
    }
    async fn list(
        &self,
        _: &str,
        _: i64,
        _: bool,
    ) -> Result<Vec<sentinel_core::domain::entities::community::confession::Confession>, DomainError>
    {
        unimplemented!()
    }
    async fn create_reply(
        &self,
        _: sentinel_api::ports::inbound::community::manage_confessions::CreateReplyCommand,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionReply, DomainError>
    {
        unimplemented!()
    }
    async fn update_reply_message_id(&self, _: uuid::Uuid, _: String) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_reply(
        &self,
        _: uuid::Uuid,
        _: String,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionReply, DomainError>
    {
        unimplemented!()
    }
    async fn list_replies(
        &self,
        _: uuid::Uuid,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::confession::ConfessionReply>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn get_reply_parent_guild(&self, _: uuid::Uuid) -> Result<String, DomainError> {
        unimplemented!()
    }
    async fn create_report(
        &self,
        _: sentinel_api::ports::inbound::community::manage_confessions::CreateReportCommand,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionReport, DomainError>
    {
        unimplemented!()
    }
    async fn get_report_guild(&self, _: uuid::Uuid) -> Result<String, DomainError> {
        unimplemented!()
    }
    async fn list_reports(
        &self,
        _: &str,
        _: Option<sentinel_core::domain::entities::community::confession::ReportStatus>,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::confession::ConfessionReport>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn resolve_report(
        &self,
        _: uuid::Uuid,
        _: sentinel_core::domain::entities::community::confession::ReportStatus,
        _: String,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get_config(
        &self,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionConfig, DomainError>
    {
        unimplemented!()
    }
    async fn save_config(
        &self,
        _: sentinel_core::domain::entities::community::confession::ConfessionConfig,
    ) -> Result<sentinel_core::domain::entities::community::confession::ConfessionConfig, DomainError>
    {
        unimplemented!()
    }
}

pub struct StubRolePanels;
#[async_trait]
impl ManageRolePanelsUseCase for StubRolePanels {
    async fn create_panel(
        &self,
        _: manage_role_panels::CreateRolePanelCommand,
    ) -> Result<RolePanelDetail, DomainError> {
        unimplemented!()
    }
    async fn get_panel(&self, _: &str) -> Result<RolePanelDetail, DomainError> {
        unimplemented!()
    }
    async fn get_panel_by_message(&self, _: &str) -> Result<Option<RolePanelDetail>, DomainError> {
        unimplemented!()
    }
    async fn list_panels(&self, _: &str) -> Result<Vec<RolePanel>, DomainError> {
        unimplemented!()
    }
    async fn set_message_id(
        &self,
        _: manage_role_panels::SetMessageIdCommand,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_panel(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn list_auto_roles(&self, _: &str) -> Result<Vec<AutoRole>, DomainError> {
        unimplemented!()
    }
    async fn add_auto_role(
        &self,
        _: manage_role_panels::CreateAutoRoleCommand,
    ) -> Result<AutoRole, DomainError> {
        unimplemented!()
    }
    async fn delete_auto_role(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

pub struct StubReminders;
#[async_trait]
impl ManageRemindersUseCase for StubReminders {
    async fn create_reminder(
        &self,
        cmd: manage_reminders::CreateReminderCommand,
    ) -> Result<SanctionReminder, DomainError> {
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
    async fn get_pending_reminders(&self) -> Result<Vec<SanctionReminder>, DomainError> {
        Ok(vec![])
    }
    async fn mark_sent(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn cancel_for_action(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_by_guild(&self, _: &str) -> Result<Vec<SanctionReminder>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubNotes;
#[async_trait]
impl ManageNotesUseCase for StubNotes {
    async fn add_note(&self, _: manage_notes::AddNoteCommand) -> Result<UserNote, DomainError> {
        unimplemented!()
    }
    async fn get_notes(&self, _: &str, _: &str) -> Result<Vec<UserNote>, DomainError> {
        Ok(vec![])
    }
    async fn delete_note(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn note_guild_id(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
}

pub struct StubStrikes;
#[async_trait]
impl ManageStrikesUseCase for StubStrikes {
    async fn add_strike(
        &self,
        cmd: manage_strikes::AddStrikeCommand,
    ) -> Result<StrikeResult, DomainError> {
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
    async fn get_active_strikes(&self, _: &str, _: &str) -> Result<Vec<UserStrike>, DomainError> {
        Ok(vec![])
    }
    async fn reset_strikes(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_config(&self, guild_id: &str) -> Result<StrikeConfig, DomainError> {
        Ok(StrikeConfig::default_for_guild(guild_id))
    }
    async fn save_config(
        &self,
        _: manage_strikes::SaveStrikeConfigCommand,
    ) -> Result<StrikeConfig, DomainError> {
        unimplemented!()
    }
}

pub struct StubModerationCopilot;
#[async_trait]
impl sentinel_api::ports::inbound::moderation::moderation_copilot::ModerationCopilotUseCase
    for StubModerationCopilot
{
    async fn get_member_context(
        &self,
        _guild_id: &str,
        _user_id: &str,
        _lookback_days: i64,
        _min_precedents: u32,
    ) -> Result<
        sentinel_core::domain::entities::moderation::copilot::MemberModerationContext,
        DomainError,
    > {
        use sentinel_core::domain::entities::moderation::copilot::*;
        Ok(MemberModerationContext {
            active_strikes: 0,
            sanctions_by_type: vec![],
            last_sanction_at: None,
            open_reviews: 0,
            precedents: PrecedentDistribution::default(),
            suggestion: SanctionSuggestion {
                action: None,
                basis: SuggestionBasis::Insufficient,
                rationale: "stub".into(),
                precedent_count: 0,
            },
        })
    }
}

// ══════════════════════════════════════════════════════════
// Stub Repositories (outbound)
// ══════════════════════════════════════════════════════════

pub struct StubAnalyticsRepo;
#[async_trait]
impl AnalyticsRepository for StubAnalyticsRepo {
    async fn get_heatmap(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<HourlyActivity>, DomainError> {
        unimplemented!()
    }
    async fn get_action_distribution(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<ActionDistribution>, DomainError> {
        unimplemented!()
    }
    async fn get_top_infractors(
        &self,
        _: Option<&str>,
        _: i32,
        _: i64,
        _: i64,
    ) -> Result<Vec<TopInfractor>, DomainError> {
        unimplemented!()
    }
    async fn get_moderation_trend(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<ModerationTrend>, DomainError> {
        unimplemented!()
    }
    async fn get_peak_hours(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<PeakActivity>, DomainError> {
        unimplemented!()
    }
    async fn record_hourly(&self, _: &str, _: i16, _: i64, _: i32) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn reset_activity(&self, _: &str) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

pub struct StubDailyActivityRepo;
#[async_trait]
impl DailyActivityRepository for StubDailyActivityRepo {
    async fn get_activity(
        &self,
        _: Option<&str>,
        _: i32,
    ) -> Result<Vec<DailyActivity>, DomainError> {
        unimplemented!()
    }
    async fn record_daily_snapshot(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

pub struct StubLogRepo;
#[async_trait]
impl LogRepository for StubLogRepo {
    async fn save(&self, _: &LogEntry) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_all(&self, _: i64) -> Result<Vec<LogEntry>, DomainError> {
        Ok(vec![])
    }
    async fn find_filtered(
        &self,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&str>,
        _: i64,
    ) -> Result<Vec<LogEntry>, DomainError> {
        Ok(vec![])
    }
    async fn delete_by_category(&self, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn delete_older_than_days(&self, _: i32) -> Result<u64, DomainError> {
        Ok(0)
    }
}

pub struct StubSystemLogs;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_system_logs::ManageSystemLogsUseCase
    for StubSystemLogs
{
    async fn list_logs(
        &self,
        _: sentinel_core::ports::inbound::system::manage_system_logs::SystemLogFilters,
    ) -> Result<Vec<LogEntry>, DomainError> {
        Ok(vec![])
    }
    async fn purge_category(&self, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
}

pub struct StubGuildRepo;
#[async_trait]
impl GuildRepository for StubGuildRepo {
    async fn upsert(&self, _: &Guild) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn find_all(&self) -> Result<Vec<Guild>, DomainError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<Guild>, DomainError> {
        unimplemented!()
    }
    async fn delete(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_absent(&self, _: &[String]) -> Result<u64, DomainError> {
        unimplemented!()
    }
}

pub struct StubBotConfigRepo;
#[async_trait]
impl BotConfigRepository for StubBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        unimplemented!()
    }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(vec![])
    }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        unimplemented!()
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

pub struct StubDiscordRoleRepo;
#[async_trait]
impl DiscordRoleRepository for StubDiscordRoleRepo {
    async fn sync_roles(&self, _: &str, _: Vec<DiscordRole>) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn find_by_guild(&self, _: &str) -> Result<Vec<DiscordRole>, DomainError> {
        unimplemented!()
    }
    async fn find_by_id(&self, _: &str, _: &str) -> Result<Option<DiscordRole>, DomainError> {
        unimplemented!()
    }
}

pub struct StubMembers;
#[async_trait]
impl ManageMembersUseCase for StubMembers {
    async fn list_members(&self, _: &str) -> Result<Vec<GuildMember>, DomainError> {
        unimplemented!()
    }
    async fn get_member(&self, _: &str, _: &str) -> Result<GuildMember, DomainError> {
        unimplemented!()
    }
    async fn get_member_summary(&self, _: &str, _: &str) -> Result<MemberSummary, DomainError> {
        unimplemented!()
    }
    async fn sync_members(
        &self,
        _: manage_members::SyncMembersCommand,
    ) -> Result<u64, DomainError> {
        unimplemented!()
    }
    async fn register_member(
        &self,
        _: manage_members::RegisterMemberCommand,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn remove_member(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_member(
        &self,
        _: manage_members::UpdateMemberCommand,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn reset_member(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Vec<(&'static str, u64)>, DomainError> {
        unimplemented!()
    }
    async fn leave_member(&self, _: &str, _: &str) -> Result<u64, DomainError> {
        unimplemented!()
    }
    async fn rejoin_member(&self, _: &str, _: &str) -> Result<u64, DomainError> {
        unimplemented!()
    }
}


// ── Stubs pour les nouveaux repos ──

pub struct StubUserActivityRepo;
#[async_trait]
impl UserActivityRepository for StubUserActivityRepo {
    async fn create(&self, _: &UserActivity) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: i64,
        _: i64,
    ) -> Result<Vec<UserActivity>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubWelcomeConfigRepo;
#[async_trait]
impl WelcomeConfigRepository for StubWelcomeConfigRepo {
    async fn get_config(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError> {
        Ok(WelcomeConfigData {
            guild_id: guild_id.into(),
            welcome_enabled: true,
            welcome_channel_id: None,
            welcome_message: String::new(),
            welcome_embed_color: "3498db".into(),
            welcome_dm_enabled: false,
            welcome_dm_message: String::new(),
            leave_enabled: false,
            leave_channel_id: None,
            leave_message: String::new(),
            rules_enabled: false,
            rules_channel_id: None,
            rules_message: String::new(),
            rules_role_id: None,
            rules_button_label: String::new(),
            age_check_enabled: false,
            age_minimum: 0,
            unverified_role_id: None,
            age_modal_question: String::new(),
            age_ban_message: String::new(),
            age_min: 5,
            age_max: 120,
            age_ban_days_per_year: 365,
            age_ban_log_channel_id: None,
            leave_embed_color: "e74c3c".into(),
            rules_embed_color: "5865f2".into(),
            counter_enabled: false,
            counter_channel_id: None,
            counter_format: String::new(),
            voice_counter_enabled: false,
            voice_counter_channel_id: None,
            voice_counter_format: String::new(),
            anniversary_enabled: false,
            anniversary_channel_id: None,
            anniversary_message: String::new(),
            rejoin_message: String::new(),
            welcome_title: String::new(),
            welcome_image_url: String::new(),
            welcome_footer_text: String::new(),
            rejoin_title: String::new(),
            rejoin_image_url: String::new(),
            rejoin_footer_text: String::new(),
            leave_title: String::new(),
            leave_image_url: String::new(),
            leave_footer_text: String::new(),
            anniversary_title: String::new(),
            anniversary_image_url: String::new(),
            anniversary_footer_text: String::new(),
        })
    }
    async fn save_config(
        &self,
        _: &str,
        d: &WelcomeConfigData,
    ) -> Result<WelcomeConfigData, DomainError> {
        Ok(d.clone())
    }
}

pub struct StubGuildResetRepo;
#[async_trait]
impl sentinel_api::ports::outbound::system::guild_reset_repository::GuildResetRepository
    for StubGuildResetRepo
{
    async fn guild_name(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn collect_discord_context(
        &self,
        _: &str,
    ) -> Result<
        sentinel_api::ports::outbound::system::guild_reset_repository::ResetDiscordContext,
        DomainError,
    > {
        Ok(Default::default())
    }
    async fn wipe_guild(&self, _: &str) -> Result<Vec<(String, u64)>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubAutomodReviewRepo;
#[async_trait]
impl sentinel_api::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository
    for StubAutomodReviewRepo
{
    async fn create(
        &self,
        _: sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn fp_terminal_reviews(
        &self,
        _: &str,
        _: i64,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::FpTerminalReview>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn create_or_merge(
        &self,
        _: sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview,
        _: bool,
        _: i64,
    ) -> Result<
        (
            sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
            bool,
        ),
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn expire_stale_decided(
        &self,
        _: i64,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::ExpiredReviewCard>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn find_discussion(
        &self,
        _: Uuid,
    ) -> Result<
        Option<sentinel_core::domain::entities::moderation::review::automod::DiscussionChannel>,
        DomainError,
    > {
        Ok(None)
    }
    async fn create_discussion(
        &self,
        _: sentinel_core::domain::entities::moderation::review::automod::NewDiscussionChannel,
    ) -> Result<
        (
            sentinel_core::domain::entities::moderation::review::automod::DiscussionChannel,
            bool,
        ),
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn delete_discussion(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn append_discussion_messages(
        &self,
        _: &[sentinel_core::domain::entities::moderation::review::automod::DiscussionMessage],
    ) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn list_discussion_messages(
        &self,
        _: Uuid,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::DiscussionMessage>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn expire_review_cards(
        &self,
        _: i64,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::ExpiredReviewCard>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn upsert_vote(&self, _: Uuid, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_votes(
        &self,
        _: Uuid,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::ReviewVote>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn decide(
        &self,
        _: Uuid,
        _: &str,
        _: bool,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn list_expired_voting(
        &self,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<
        Option<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(None)
    }
    async fn find_by_message_id(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Option<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(None)
    }
    async fn list_pending(
        &self,
        _: &str,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn list_recent(
        &self,
        _: &str,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::moderation::review::automod::AutomodReview>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn resolve(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn close_ignored(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn reopen(
        &self,
        _: Uuid,
        _: i64,
    ) -> Result<
        sentinel_core::domain::entities::moderation::review::automod::AutomodReview,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
}

pub struct StubDiscordActionMessageRepo;
#[async_trait]
impl sentinel_api::ports::outbound::audit::discord_action_message_repository::DiscordActionMessageRepository for StubDiscordActionMessageRepo {
    async fn register(&self, _: sentinel_core::domain::entities::audit::discord_action_message::NewDiscordActionMessage) -> Result<(), DomainError> { Ok(()) }
    async fn list_for_action(&self, _: Uuid) -> Result<Vec<sentinel_core::domain::entities::audit::discord_action_message::DiscordActionMessage>, DomainError> { Ok(vec![]) }
}

pub struct StubExportUC;
#[async_trait]
impl sentinel_api::application::system::export_service::ExecuteExportUseCase for StubExportUC {
    async fn execute(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<sentinel_api::application::system::export_service::ExportResult, DomainError> {
        Ok(
            sentinel_api::application::system::export_service::ExportResult {
                data: String::new(),
                row_count: 0,
            },
        )
    }
}

pub struct StubExportJobsUC;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_export_jobs::ManageExportJobsUseCase
    for StubExportJobsUC
{
    async fn enqueue(
        &self,
        _: sentinel_core::ports::outbound::system::export_job_repository::NewExportJob,
    ) -> Result<Uuid, DomainError> {
        Ok(Uuid::new_v4())
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<
        Option<sentinel_core::ports::outbound::system::export_job_repository::ExportJobRecord>,
        DomainError,
    > {
        Ok(None)
    }
}

pub struct StubEvidenceRepo;
#[async_trait]
impl EvidenceRepository for StubEvidenceRepo {
    async fn add(
        &self,
        _: Uuid,
        _: &str,
        _: Option<&str>,
        _: &str,
        _: &str,
    ) -> Result<EvidenceEntry, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn list(&self, _: Uuid) -> Result<Vec<EvidenceEntry>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubReviewRepo;
#[async_trait]
impl ReviewRepository for StubReviewRepo {
    async fn add(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<ReviewEntry, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn list_pending(&self, _: &str) -> Result<Vec<ReviewEntry>, DomainError> {
        Ok(vec![])
    }
    async fn resolve(
        &self,
        _: Uuid,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: &str,
    ) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn get_guild_id(&self, _: Uuid) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
}

pub struct StubModstatsRepo;
#[async_trait]
impl ModstatsRepository for StubModstatsRepo {
    async fn top_moderators(
        &self,
        _: &str,
        _: i32,
        _: i64,
    ) -> Result<Vec<ModeratorStat>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubGameRepo;
#[async_trait]
impl GameRepository for StubGameRepo {
    async fn list(&self, _: &str) -> Result<Vec<Game>, DomainError> {
        Ok(vec![])
    }
    async fn list_by_category(&self, _: &str, _: Option<&str>) -> Result<Vec<Game>, DomainError> {
        Ok(vec![])
    }
    async fn create(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<Game, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn update(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<Option<&str>>,
        _: Option<Option<&str>>,
    ) -> Result<Option<Game>, DomainError> {
        Ok(None)
    }
    async fn delete(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn find_by_name(&self, _: &str, _: &str) -> Result<Option<Game>, DomainError> {
        Ok(None)
    }
    async fn set_role_id(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<Option<Game>, DomainError> {
        Ok(None)
    }
    async fn save_panel(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<GamePanel, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn find_panel_by_message(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<GamePanel>, DomainError> {
        Ok(None)
    }
    async fn list_panels(&self, _: &str) -> Result<Vec<GamePanel>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubSponsorshipRepo;
#[async_trait]
impl SponsorshipRepository for StubSponsorshipRepo {
    async fn create(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list(&self, _: &str) -> Result<Vec<Sponsorship>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubTempRoleRepo;
#[async_trait]
impl TempRoleRepository for StubTempRoleRepo {
    async fn create(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_active(&self, _: &str) -> Result<Vec<TempRole>, DomainError> {
        Ok(vec![])
    }
    async fn delete(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubPendingActionRepo;
#[async_trait]
impl PendingActionRepository for StubPendingActionRepo {
    async fn create(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<i64>,
    ) -> Result<Uuid, DomainError> {
        Ok(Uuid::new_v4())
    }
    async fn list_pending(&self, _: &str) -> Result<Vec<PendingAction>, DomainError> {
        Ok(vec![])
    }
    async fn get_guild_id(&self, _: Uuid) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn resolve(&self, _: Uuid, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}


// ── Stubs Tamagotchi / Systeme securite / Game Portal ──

pub struct StubGuildSnapshots;
#[async_trait]
impl sentinel_core::ports::inbound::guild_backup::manage_snapshots::ManageGuildSnapshotsUseCase
    for StubGuildSnapshots
{
    async fn store_snapshot(
        &self,
        _: sentinel_core::domain::entities::guild_backup::snapshot::GuildSnapshot,
    ) -> Result<
        sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
        DomainError,
    > {
        unimplemented!()
    }
    async fn store_snapshot_with_quota(
        &self,
        _: sentinel_core::domain::entities::guild_backup::snapshot::GuildSnapshot,
        _: u32,
    ) -> Result<
        sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
        DomainError,
    > {
        unimplemented!()
    }
    async fn list_snapshots(
        &self,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotSummary>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn get_snapshot(
        &self,
        _: sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
    ) -> Result<
        sentinel_core::domain::entities::guild_backup::snapshot::GuildSnapshot,
        DomainError,
    > {
        unimplemented!()
    }
    async fn delete_snapshot(
        &self,
        _: sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
    ) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn rename_snapshot(
        &self,
        _: sentinel_core::ports::inbound::guild_backup::manage_snapshots::SnapshotId,
        _: &str,
    ) -> Result<bool, DomainError> {
        unimplemented!()
    }
}

pub struct StubPendingRoleGrants;
#[async_trait]
impl sentinel_core::ports::inbound::guild_backup::manage_pending_role_grants::ManagePendingRoleGrantsUseCase
    for StubPendingRoleGrants
{
    async fn save_grants(
        &self,
        _: &str,
        _: Vec<sentinel_core::domain::entities::guild_backup::pending_role_grant::PendingRoleGrant>,
    ) -> Result<u64, DomainError> {
        unimplemented!()
    }
    async fn take_grant(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<Vec<String>>, DomainError> {
        unimplemented!()
    }
    async fn clear_guild(&self, _: &str) -> Result<u64, DomainError> {
        unimplemented!()
    }
}


pub struct StubRotation;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_rotation::ManageRotationUseCase
    for StubRotation
{
    async fn get_state(
        &self,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::system::admin_rotation::RotationState, DomainError>
    {
        unimplemented!()
    }
    async fn save_state(
        &self,
        _: sentinel_core::domain::entities::system::admin_rotation::RotationState,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn record_served(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn served_entries(
        &self,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::admin_rotation::ServedEntry>,
        DomainError,
    > {
        unimplemented!()
    }
}

pub struct StubIpBans;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_ip_bans::ManageIpBansUseCase for StubIpBans {
    async fn ban(
        &self,
        _: &str,
        _: Option<String>,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::system::ip_ban::BanIpOutcome, DomainError> {
        unimplemented!()
    }
    async fn unban(&self, _: &str, _: Option<String>, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn list_manual_bans(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::system::ip_ban::ManualIpBan>, DomainError>
    {
        unimplemented!()
    }
    async fn fail2ban_status(
        &self,
    ) -> Result<Option<sentinel_core::domain::entities::system::ip_ban::Fail2banStatus>, DomainError>
    {
        unimplemented!()
    }
}

pub struct StubHostProbe;
#[async_trait]
impl sentinel_core::ports::inbound::system::read_host_probe::ReadHostProbeUseCase
    for StubHostProbe
{
    async fn read(
        &self,
        _: sentinel_core::domain::entities::system::host_probe::HostProbe,
    ) -> Result<serde_json::Value, DomainError> {
        unimplemented!()
    }
}

pub struct StubSecurityLogs;
#[async_trait]
impl sentinel_core::ports::inbound::system::read_security_logs::ReadSecurityLogsUseCase
    for StubSecurityLogs
{
    async fn top_ips(
        &self,
        _: sentinel_core::domain::entities::system::security_log::LogWindow,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::system::security_log::TopIp>, DomainError>
    {
        unimplemented!()
    }
    async fn auth_failures(
        &self,
        _: sentinel_core::domain::entities::system::security_log::LogWindow,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::system::security_log::AuthFailure>, DomainError>
    {
        unimplemented!()
    }
    async fn traffic_trend(
        &self,
        _: sentinel_core::domain::entities::system::security_log::LogWindow,
        _: i64,
    ) -> Result<sentinel_core::domain::entities::system::security_log::TrafficTrend, DomainError>
    {
        unimplemented!()
    }
}

pub struct StubSecurityAudit;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_security_audit::ManageSecurityAuditUseCase
    for StubSecurityAudit
{
    async fn audit_logs(
        &self,
        _: sentinel_core::domain::entities::system::security_audit::AuditLogFilter,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::security_audit::AuditLogEntry>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn recent_logins(
        &self,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::security_audit::SuccessfulLogin>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn cleanup(
        &self,
        _: sentinel_core::domain::entities::system::security_audit::CleanupOptions,
    ) -> Result<sentinel_core::domain::entities::system::security_audit::CleanupReport, DomainError>
    {
        unimplemented!()
    }
}

pub struct StubTlsCert;
#[async_trait]
impl sentinel_core::ports::inbound::system::read_tls_cert::ReadTlsCertUseCase for StubTlsCert {
    async fn read(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::tls_cert::TlsCertInfo, DomainError> {
        unimplemented!()
    }
}

pub struct StubGeoIp;
#[async_trait]
impl sentinel_core::ports::inbound::system::lookup_geoip::LookupGeoIpUseCase for StubGeoIp {
    async fn lookup(
        &self,
        _: Vec<String>,
    ) -> Result<Vec<sentinel_core::domain::entities::system::geoip::GeoIpEntry>, DomainError> {
        unimplemented!()
    }
}

pub struct StubGameServers;
#[async_trait]
impl sentinel_core::ports::inbound::game::manage_game_servers::ManageGameServersUseCase
    for StubGameServers
{
    async fn create(
        &self,
        _: sentinel_core::domain::entities::game::server::CreateGameServerCommand,
    ) -> Result<sentinel_core::domain::entities::game::server::GameServer, DomainError> {
        unimplemented!()
    }
    async fn list_for_guild(
        &self,
        _: &str,
    ) -> Result<Vec<sentinel_core::domain::entities::game::server::GameServer>, DomainError> {
        unimplemented!()
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<
        sentinel_core::ports::inbound::game::manage_game_servers::GameServerDetail,
        DomainError,
    > {
        unimplemented!()
    }
    async fn delete(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn start(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn stop(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn restart(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get_logs(&self, _: Uuid, _: u32) -> Result<Vec<String>, DomainError> {
        unimplemented!()
    }
    async fn get_stats(
        &self,
        _: Uuid,
    ) -> Result<sentinel_core::ports::outbound::game::container_runtime::ContainerStats, DomainError>
    {
        unimplemented!()
    }
    async fn update_config(
        &self,
        _: Uuid,
        _: std::collections::HashMap<String, String>,
        _: &str,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn execute_rcon(&self, _: Uuid, _: &str, _: &str) -> Result<String, DomainError> {
        unimplemented!()
    }
}

pub struct StubGameTemplates;
#[async_trait]
impl sentinel_core::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase
    for StubGameTemplates
{
    async fn list_for_guild(
        &self,
        _: &str,
    ) -> Result<Vec<sentinel_core::domain::entities::game::template::GameTemplate>, DomainError>
    {
        unimplemented!()
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<sentinel_core::domain::entities::game::template::GameTemplate, DomainError> {
        unimplemented!()
    }
    async fn get_by_slug(
        &self,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::game::template::GameTemplate, DomainError> {
        unimplemented!()
    }
}

pub struct StubGameServerRepo;
#[async_trait]
impl sentinel_core::ports::outbound::game::game_server_repository::GameServerRepository
    for StubGameServerRepo
{
    async fn create(
        &self,
        _: sentinel_core::ports::outbound::game::game_server_repository::NewGameServer,
    ) -> Result<sentinel_core::domain::entities::game::server::GameServer, DomainError> {
        unimplemented!()
    }
    async fn find_by_id(
        &self,
        _: Uuid,
    ) -> Result<Option<sentinel_core::domain::entities::game::server::GameServer>, DomainError>
    {
        unimplemented!()
    }
    async fn list_by_guild(
        &self,
        _: &str,
    ) -> Result<Vec<sentinel_core::domain::entities::game::server::GameServer>, DomainError> {
        unimplemented!()
    }
    async fn list_running(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::game::server::GameServer>, DomainError> {
        unimplemented!()
    }
    async fn list_active(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::game::server::GameServer>, DomainError> {
        unimplemented!()
    }
    async fn update_runtime(
        &self,
        _: Uuid,
        _: sentinel_core::ports::outbound::game::game_server_repository::GameServerRuntimeUpdate,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn update_status(
        &self,
        _: Uuid,
        _: sentinel_core::domain::entities::game::server::GameServerStatus,
        _: Option<&str>,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn try_transition_status(
        &self,
        _: Uuid,
        _: &[sentinel_core::domain::entities::game::server::GameServerStatus],
        _: sentinel_core::domain::entities::game::server::GameServerStatus,
    ) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn update_player_activity(&self, _: Uuid, _: i32) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn record_restart_attempt(&self, _: Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn reset_restart_attempts(&self, _: Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn soft_delete(&self, _: Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn count_active_for_guild(&self, _: &str) -> Result<(i32, i32), DomainError> {
        unimplemented!()
    }
    async fn template_usage(
        &self,
        _: uuid::Uuid,
    ) -> Result<
        sentinel_core::ports::outbound::game::game_server_repository::TemplateUsage,
        DomainError,
    > {
        unimplemented!()
    }
    async fn set_session_channels(
        &self,
        _: Uuid,
        _: Option<&str>,
        _: Option<&str>,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn mark_ip_revealed(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_ip_reveal_due(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::game::server::GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn list_awaiting_reveal_no_ping_today(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::game::server::GameServer>, DomainError> {
        Ok(vec![])
    }
    async fn mark_daily_ping(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn set_ip_reveal_at(
        &self,
        _: Uuid,
        _: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubGameTemplateRepo;
#[async_trait]
impl sentinel_core::ports::outbound::game::game_template_repository::GameTemplateRepository
    for StubGameTemplateRepo
{
    async fn list(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::game::template::GameTemplate>, DomainError>
    {
        unimplemented!()
    }
    async fn find_by_id(
        &self,
        _: Uuid,
    ) -> Result<Option<sentinel_core::domain::entities::game::template::GameTemplate>, DomainError>
    {
        unimplemented!()
    }
    async fn find_by_slug(
        &self,
        _: &str,
    ) -> Result<Option<sentinel_core::domain::entities::game::template::GameTemplate>, DomainError>
    {
        unimplemented!()
    }
}

pub struct StubGameAuditRepo;
#[async_trait]
impl sentinel_core::ports::outbound::game::game_audit_repository::GameAuditRepository
    for StubGameAuditRepo
{
    async fn log(
        &self,
        _: &str,
        _: Option<Uuid>,
        _: Option<&str>,
        _: sentinel_core::domain::entities::game::audit::GameAuditAction,
        _: serde_json::Value,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn list_for_server(
        &self,
        _: Uuid,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::game::audit::GameAuditEntry>, DomainError>
    {
        unimplemented!()
    }
    async fn list_for_guild(
        &self,
        _: &str,
        _: i64,
        _: i64,
    ) -> Result<Vec<sentinel_core::domain::entities::game::audit::GameAuditEntry>, DomainError>
    {
        unimplemented!()
    }
}

pub struct StubGameSessionRepo;
#[async_trait]
impl sentinel_core::ports::outbound::game::player_session_repository::PlayerSessionRepository
    for StubGameSessionRepo
{
    async fn open(&self, _: Uuid, _: &str) -> Result<Uuid, DomainError> {
        unimplemented!()
    }
    async fn close(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn list_active(
        &self,
        _: Uuid,
    ) -> Result<
        Vec<sentinel_core::domain::entities::game::player_session::PlayerSession>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn list_history(
        &self,
        _: Uuid,
        _: i64,
        _: i64,
    ) -> Result<
        Vec<sentinel_core::domain::entities::game::player_session::PlayerSession>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn close_all_active(&self, _: Uuid) -> Result<(), DomainError> {
        unimplemented!()
    }
}

pub struct StubContainerRuntime;
#[async_trait]
impl sentinel_core::ports::outbound::game::container_runtime::ContainerRuntime
    for StubContainerRuntime
{
    async fn ensure_network(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn ensure_volume(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn pull_image_if_missing(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn create_container(
        &self,
        _: &sentinel_core::ports::outbound::game::container_runtime::ContainerSpec,
    ) -> Result<String, DomainError> {
        unimplemented!()
    }
    async fn start_container(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn upload_file_to_container(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn stop_container(&self, _: &str, _: u32) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn restart_container(&self, _: &str, _: u32) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn remove_container(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn remove_volume(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn remove_image(&self, _: &str, _: bool) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn inspect(
        &self,
        _: &str,
    ) -> Result<
        Option<sentinel_core::ports::outbound::game::container_runtime::ContainerStatus>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn stats(
        &self,
        _: &str,
    ) -> Result<sentinel_core::ports::outbound::game::container_runtime::ContainerStats, DomainError>
    {
        unimplemented!()
    }
    async fn logs(&self, _: &str, _: u32) -> Result<Vec<String>, DomainError> {
        unimplemented!()
    }
    async fn list_managed_containers(
        &self,
    ) -> Result<
        Vec<sentinel_core::ports::outbound::game::container_runtime::ManagedContainer>,
        DomainError,
    > {
        unimplemented!()
    }
}

pub struct StubRconClient;
#[async_trait]
impl sentinel_core::ports::outbound::game::rcon_client::RconClient for StubRconClient {
    async fn execute(
        &self,
        _: &sentinel_core::ports::outbound::game::rcon_client::RconConnectionParams,
        _: &str,
    ) -> Result<sentinel_core::ports::outbound::game::rcon_client::RconResponse, DomainError> {
        unimplemented!()
    }
}

pub struct StubPortAllocator;
#[async_trait]
impl sentinel_core::ports::outbound::game::port_allocator::PortAllocator for StubPortAllocator {
    async fn allocate(
        &self,
        _: sentinel_core::ports::outbound::game::port_allocator::PortKind,
        _: u16,
        _: u16,
        _: &str,
    ) -> Result<u16, DomainError> {
        unimplemented!()
    }
    async fn release(
        &self,
        _: sentinel_core::ports::outbound::game::port_allocator::PortKind,
        _: u16,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn is_available(
        &self,
        _: sentinel_core::ports::outbound::game::port_allocator::PortKind,
        _: u16,
    ) -> Result<bool, DomainError> {
        unimplemented!()
    }
}

// ══════════════════════════════════════════════════════════
// Stubs additionnels (champs AppState recents)
// ══════════════════════════════════════════════════════════

pub struct StubBump;
#[async_trait]
impl sentinel_core::ports::inbound::community::manage_bump::ManageBumpUseCase for StubBump {
    async fn record_bump(
        &self,
        _: sentinel_core::ports::inbound::community::manage_bump::RecordBumpCommand,
    ) -> Result<sentinel_core::domain::entities::community::bump::BumpReward, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn due_reminders(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::community::bump::DueReminder>, DomainError>
    {
        Ok(vec![])
    }
    async fn mark_reminder_sent(
        &self,
        _: &str,
        _: Option<String>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubEligibility;
#[async_trait]
impl sentinel_core::ports::inbound::community::check_eligibility::CheckEligibilityUseCase
    for StubEligibility
{
    async fn check_role_eligibility(
        &self,
        _: sentinel_core::ports::inbound::community::check_eligibility::CheckRoleEligibilityCommand,
    ) -> Result<
        sentinel_core::domain::entities::community::eligibility::EligibilityDecision,
        DomainError,
    > {
        Ok(sentinel_core::domain::entities::community::eligibility::EligibilityDecision::allow())
    }
    async fn validate_sponsorship(
        &self,
        _: sentinel_core::ports::inbound::community::check_eligibility::ValidateSponsorshipCommand,
    ) -> Result<
        sentinel_core::domain::entities::community::eligibility::EligibilityDecision,
        DomainError,
    > {
        Ok(sentinel_core::domain::entities::community::eligibility::EligibilityDecision::allow())
    }
}

pub struct StubDataset;
#[async_trait]
impl sentinel_core::ports::inbound::ai::manage_dataset::ManageDatasetUseCase for StubDataset {
    async fn collect_message(
        &self,
        _: sentinel_core::ports::outbound::ai::dataset_repository::NewDatasetMessage,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_messages(
        &self,
        _: sentinel_core::ports::inbound::ai::manage_dataset::ListDatasetQuery,
    ) -> Result<sentinel_core::domain::entities::ai::dataset::DatasetPage, DomainError> {
        Ok(sentinel_core::domain::entities::ai::dataset::DatasetPage {
            items: vec![],
            total: 0,
        })
    }
    async fn bulk_delete(
        &self,
        _: sentinel_core::ports::inbound::ai::manage_dataset::BulkDeleteCommand,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }
}

pub struct StubAiJobs;
#[async_trait]
impl sentinel_core::ports::inbound::ai::manage_ai_jobs::ManageAiJobsUseCase for StubAiJobs {
    async fn create_job(
        &self,
        _: sentinel_core::domain::entities::ai::ai_job::NewAiJob,
    ) -> Result<Uuid, DomainError> {
        Ok(Uuid::new_v4())
    }
    async fn get_job(
        &self,
        _: Uuid,
    ) -> Result<sentinel_core::domain::entities::ai::ai_job::AiJob, DomainError> {
        Err(DomainError::NotFound("ai_job stub".into()))
    }
}

pub struct StubInvitations;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_invitations::ManageInvitationsUseCase
    for StubInvitations
{
    async fn create_invitation(
        &self,
        _: sentinel_core::ports::inbound::system::manage_invitations::CreateInvitationCommand,
    ) -> Result<sentinel_core::domain::entities::system::invitation::Invitation, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn list_invitations(
        &self,
        _: &str,
    ) -> Result<Vec<sentinel_core::domain::entities::system::invitation::Invitation>, DomainError>
    {
        Ok(vec![])
    }
    async fn find_invitation(
        &self,
        _: &str,
    ) -> Result<Option<sentinel_core::domain::entities::system::invitation::Invitation>, DomainError>
    {
        Ok(None)
    }
    async fn revoke_invitation(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn check_access(
        &self,
        _: &str,
        _: bool,
    ) -> Result<sentinel_core::domain::entities::system::invitation::AccessStatus, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn redeem_invitation(
        &self,
        _: &str,
        _: &str,
    ) -> Result<sentinel_core::domain::entities::system::invitation::RedeemedInvitation, DomainError>
    {
        Err(DomainError::Internal("stub".into()))
    }
}

pub struct StubOAuth;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_oauth::ManageOAuthUseCase for StubOAuth {
    async fn record_login(
        &self,
        _: sentinel_core::domain::entities::system::oauth::LoginTrace,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn create_session(
        &self,
        _: sentinel_core::domain::entities::system::oauth::NewOAuthSession,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_session(
        &self,
        _: Uuid,
    ) -> Result<Option<sentinel_core::domain::entities::system::oauth::OAuthSession>, DomainError>
    {
        Ok(None)
    }
    async fn touch_session(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn update_tokens(
        &self,
        _: sentinel_core::domain::entities::system::oauth::SessionTokenUpdate,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_session(&self, _: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubQuarantine;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase
    for StubQuarantine
{
    async fn quarantine_user(&self, _: &str, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_active(
        &self,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::quarantine::ActiveQuarantine>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn lift(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubComponentMinRole;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_component_min_role::ManageComponentMinRoleUseCase
    for StubComponentMinRole
{
    async fn list_overrides(
        &self,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::component_min_role::ComponentMinRoleOverride>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn get_override(&self, _: &str, _: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn upsert(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubAlertRules;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_alert_rules::ManageAlertRulesUseCase
    for StubAlertRules
{
    async fn list(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::system::alert_rule::AlertRule>, DomainError>
    {
        Ok(vec![])
    }
    async fn update(
        &self,
        _: &str,
        _: sentinel_core::domain::entities::system::alert_rule::AlertRuleUpdate,
    ) -> Result<sentinel_core::domain::entities::system::alert_rule::AlertRule, DomainError> {
        Err(DomainError::NotFound("regle d'alerte inconnue".into()))
    }
}

pub struct StubSystemProbe;
#[async_trait]
impl sentinel_core::ports::outbound::system::system_probe::SystemProbe for StubSystemProbe {}

pub struct StubDockerHost;
#[async_trait]
impl sentinel_core::ports::outbound::system::docker_host::DockerHost for StubDockerHost {
    async fn version_info(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::DockerVersionInfo, DomainError>
    {
        Ok(Default::default())
    }
    async fn disk_usage(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::DiskUsage, DomainError> {
        Ok(Default::default())
    }
    async fn list_containers(
        &self,
        _: bool,
    ) -> Result<Vec<sentinel_core::domain::entities::system::docker_host::ContainerSummary>, DomainError>
    {
        Ok(vec![])
    }
    async fn start_container(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn stop_container(&self, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn restart_container(&self, _: &str, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn remove_container(&self, _: &str, _: bool, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn container_logs(&self, _: &str, _: u32, _: bool) -> Result<String, DomainError> {
        Ok(String::new())
    }
    async fn list_images(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::system::docker_host::ImageSummary>, DomainError>
    {
        Ok(vec![])
    }
    async fn remove_image(&self, _: &str, _: bool, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_volumes(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::system::docker_host::VolumeSummary>, DomainError>
    {
        Ok(vec![])
    }
    async fn remove_volume(&self, _: &str, _: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_networks(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::system::docker_host::NetworkSummary>, DomainError>
    {
        Ok(vec![])
    }
    async fn prune_containers(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
    async fn prune_images(
        &self,
        _: bool,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
    async fn prune_volumes(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
    async fn prune_networks(
        &self,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
    async fn prune_build_cache(
        &self,
        _: bool,
    ) -> Result<sentinel_core::domain::entities::system::docker_host::PruneOutcome, DomainError>
    {
        Ok(Default::default())
    }
}

pub struct StubLockdown;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_lockdown::ManageLockdownUseCase
    for StubLockdown
{
    async fn activate(
        &self,
        _: &str,
        _: serde_json::Value,
        _: i64,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn deactivate(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubSlowmode;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_slowmode::ManageSlowmodeUseCase
    for StubSlowmode
{
    async fn activate(
        &self,
        _: &str,
        _: serde_json::Value,
        _: i64,
        _: i32,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn deactivate(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubComponentVisibility;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_component_visibility::ManageComponentVisibilityUseCase
    for StubComponentVisibility
{
    async fn list(
        &self,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::component_visibility::VisibilityEntry>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn upsert_batch(
        &self,
        _: &str,
        _: Vec<sentinel_core::domain::entities::system::component_visibility::VisibilityEntry>,
        _: &str,
    ) -> Result<usize, DomainError> {
        Ok(0)
    }
}

pub struct StubBotPersistence;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase
    for StubBotPersistence
{
    async fn update_streak(
        &self,
        _: &str,
        _: &str,
        _: i32,
        _: i32,
        _: i32,
        _: i32,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubServerEvents;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_server_events::ManageServerEventsUseCase
    for StubServerEvents
{
    async fn record(
        &self,
        _: &str,
        _: Option<&str>,
        _: &str,
        _: Option<&str>,
        _: &str,
        _: serde_json::Value,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list(
        &self,
        _: Option<String>,
        _: Option<String>,
        _: Option<i64>,
    ) -> Result<
        Vec<sentinel_core::domain::entities::system::server_event::ServerEvent>,
        DomainError,
    > {
        Ok(vec![])
    }
}

pub struct StubRbac;
#[async_trait]
impl sentinel_core::ports::inbound::system::manage_rbac::ManageRbacUseCase for StubRbac {
    async fn grant_role(
        &self,
        _: sentinel_core::ports::inbound::system::manage_rbac::GrantRoleCommand,
    ) -> Result<sentinel_core::domain::entities::system::rbac::UserRoleGrant, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn update_role(
        &self,
        _: sentinel_core::ports::inbound::system::manage_rbac::UpdateRoleCommand,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn revoke_role(
        &self,
        _: sentinel_core::ports::inbound::system::manage_rbac::RevokeRoleCommand,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_guild_users(
        &self,
        _: &str,
    ) -> Result<Vec<sentinel_core::domain::entities::system::rbac::GuildUserEntry>, DomainError>
    {
        Ok(vec![])
    }
    async fn ensure_owner_grant(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn is_whitelisted(&self, _: &str) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn role_for_guild(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Option<sentinel_core::domain::enums::system::role::Role>, DomainError> {
        Ok(None)
    }
    async fn record_user_seen(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubMonthlyRanking;
#[async_trait]
impl sentinel_core::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase
    for StubMonthlyRanking
{
    async fn force_ranking(
        &self,
        _: &str,
        _: Option<String>,
    ) -> Result<
        sentinel_core::domain::entities::community::monthly_ranking::MonthlyRankingData,
        DomainError,
    > {
        Err(DomainError::Internal("stub".into()))
    }
    async fn plan_and_baseline(
        &self,
    ) -> Result<
        sentinel_core::domain::entities::community::monthly_ranking::MonthlyPublishPlan,
        DomainError,
    > {
        Ok(Default::default())
    }
    async fn mark_published(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}


pub struct StubSursis;
#[async_trait]
impl sentinel_core::ports::inbound::moderation::manage_sursis::ManageSursisUseCase for StubSursis {
    async fn create(
        &self,
        _: sentinel_core::ports::inbound::moderation::manage_sursis::CreateSursisCommand,
    ) -> Result<sentinel_core::domain::entities::moderation::sursis::Sursis, DomainError> {
        Err(DomainError::Internal("stub".into()))
    }
    async fn get(
        &self,
        _: Uuid,
    ) -> Result<Option<sentinel_core::domain::entities::moderation::sursis::Sursis>, DomainError>
    {
        Ok(None)
    }
    async fn resolve(
        &self,
        _: Uuid,
        _: sentinel_core::domain::entities::moderation::sursis::SursisStatus,
    ) -> Result<bool, DomainError> {
        Ok(true)
    }
    async fn list_due(
        &self,
    ) -> Result<Vec<sentinel_core::domain::entities::moderation::sursis::Sursis>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubAdaptiveSlowmodeRepo;
#[async_trait]
impl sentinel_core::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository
    for StubAdaptiveSlowmodeRepo
{
    async fn mark(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unmark(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_all(&self) -> Result<Vec<(String, String)>, DomainError> {
        Ok(vec![])
    }
}

pub struct StubGameTemplateSettingsRepo;
#[async_trait]
impl sentinel_core::ports::outbound::game::game_session_repository::GameTemplateSettingsRepository
    for StubGameTemplateSettingsRepo
{
    async fn get(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Option<sentinel_core::domain::entities::game::session::GameTemplateSettings>,
        DomainError,
    > {
        Ok(None)
    }
    async fn list(
        &self,
        _: &str,
    ) -> Result<Vec<sentinel_core::domain::entities::game::session::GameTemplateSettings>, DomainError>
    {
        Ok(vec![])
    }
    async fn set_role(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

pub struct StubGameSessionRegRepo;
#[async_trait]
impl sentinel_core::ports::outbound::game::game_session_repository::GameSessionRegistrationRepository
    for StubGameSessionRegRepo
{
    async fn register(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn unregister(&self, _: Uuid, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list(
        &self,
        _: Uuid,
    ) -> Result<Vec<sentinel_core::domain::entities::game::session::GameSessionRegistration>, DomainError>
    {
        Ok(vec![])
    }
}

// ══════════════════════════════════════════════════════════
// TestAppState builder
// ══════════════════════════════════════════════════════════

struct StubSponsorships;
#[async_trait]
impl sentinel_api::ports::inbound::community::manage_sponsorships::ManageSponsorshipsUseCase
    for StubSponsorships
{
    async fn create_sponsorship(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_sponsorships(&self, _: &str) -> Result<Vec<Sponsorship>, DomainError> {
        Ok(vec![])
    }
    async fn create_temp_role(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn list_temp_roles(&self, _: &str) -> Result<Vec<TempRole>, DomainError> {
        Ok(vec![])
    }
    async fn delete_temp_role(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

/// Construit un AppState de base avec tous les stubs.
fn base_state() -> AppState {
    // On branche sur le compose de test (6380/5433) pour que les branches
    // redis/sqlx direct des handlers (caches, api_user_guilds, modstats, etc.)
    // soient reellement executees pendant les tests d'integration HTTP.
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6380".to_string());
    let redis_client = redis::Client::open(redis_url).unwrap();
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".to_string()
    });
    let pg_pool = sqlx::PgPool::connect_lazy(&db_url).unwrap();

    AppState {
        analyze_uc: Arc::new(StubAnalyzeMessage),
        analyze_image_uc: Arc::new(StubAnalyzeImage),
        rules_uc: Arc::new(StubRules),
        infractions_uc: Arc::new(StubInfractions),
        tickets_uc: Arc::new(StubTickets),
        security_uc: Arc::new(StubSecurity),
        moderation_uc: Arc::new(StubModeration),
        modstats_uc: Arc::new(sentinel_core::application::moderation::read_modstats_service::ReadModstatsService::new(Arc::new(StubModstatsRepo))),
        stats_uc: Arc::new(StubStats),
        voice_channels_uc: Arc::new(StubVoiceChannels),
        watched_users_uc: Arc::new(StubWatchedUsers),
        audit_logs_uc: Arc::new(StubAuditLogs),
        detect_anomaly_uc: Arc::new(
            sentinel_core::application::audit::detect_moderation_anomaly_service::DetectModerationAnomalyService::new(
                Arc::new(sentinel_api::adapters::outbound::audit::in_memory_anomaly_counter::InMemoryAnomalyCounter::new(500, 100)),
            ),
        ),
        weekly_report_uc: Arc::new(
            sentinel_core::application::audit::get_weekly_report_service::GetWeeklyReportService::new(
                Arc::new(StubAuditEventCounter),
            ),
        ),
        snapshots_uc: Arc::new(StubSnapshots),
        levels_uc: Arc::new(StubLevels),
        announcements_uc: Arc::new(StubAnnouncements),
        confessions_uc: Arc::new(StubConfessions),
        role_panels_uc: Arc::new(StubRolePanels),
        notes_uc: Arc::new(StubNotes),
        reminders_uc: Arc::new(StubReminders),
        strikes_uc: Arc::new(StubStrikes),
        moderation_copilot_uc: Arc::new(StubModerationCopilot),
        assess_target_risk_uc: Arc::new(
            sentinel_api::application::moderation::assess_target_risk_service::AssessTargetRiskService::new(
                Arc::new(StubBotConfigRepo),
            ),
        ),
        analytics_repo: Arc::new(StubAnalyticsRepo),
        daily_activity_repo: Arc::new(StubDailyActivityRepo),
        age_ban_repo: Arc::new(
            sentinel_api::adapters::outbound::postgres::community::age_ban_repository::PgAgeBanRepository::new(
                pg_pool.clone(),
            ),
        ),
        log_repo: Arc::new(StubLogRepo),
        system_logs_uc: Arc::new(StubSystemLogs),
        guild_repo: Arc::new(StubGuildRepo),
        bot_config_repo: Arc::new(StubBotConfigRepo),
        discord_role_repo: Arc::new(StubDiscordRoleRepo),
        members_uc: Arc::new(StubMembers),
        user_activity_repo: Arc::new(StubUserActivityRepo),
        welcome_config_uc: Arc::new(
            sentinel_api::application::community::manage_welcome_config_service::ManageWelcomeConfigService::new(
                Arc::new(StubWelcomeConfigRepo),
            ),
        ),
        age_check_uc: Arc::new(
            sentinel_api::application::community::evaluate_age_declaration_service::EvaluateAgeDeclarationService::new(
                Arc::new(StubWelcomeConfigRepo),
            ),
        ),
        automod_reviews_uc: Arc::new(
            sentinel_api::application::moderation::manage_automod_reviews_service::ManageAutomodReviewsService::new(
                Arc::new(StubAutomodReviewRepo),
            ),
        ),
        reset_guild_uc: Arc::new(
            sentinel_api::application::system::reset_guild_service::ResetGuildService::new(
                Arc::new(StubGuildResetRepo),
            ),
        ),
        discord_action_messages_uc: Arc::new(
            sentinel_api::application::audit::manage_discord_action_messages_service::ManageDiscordActionMessagesService::new(
                Arc::new(StubDiscordActionMessageRepo),
            ),
        ),
        export_uc: Arc::new(StubExportUC),
        export_jobs_uc: Arc::new(StubExportJobsUC),
        evidence_repo: Arc::new(StubEvidenceRepo),
        review_repo: Arc::new(StubReviewRepo),
        modstats_repo: Arc::new(StubModstatsRepo),
        game_repo: Arc::new(StubGameRepo),
        sponsorship_repo: Arc::new(StubSponsorshipRepo),
        temp_role_repo: Arc::new(StubTempRoleRepo),
        pending_action_repo: Arc::new(StubPendingActionRepo),
        guild_snapshots_uc: Arc::new(StubGuildSnapshots),
        pending_role_grants_uc: Arc::new(StubPendingRoleGrants),
        rotation_uc: Arc::new(StubRotation),
        ip_bans_uc: Arc::new(StubIpBans),
        host_probe_uc: Arc::new(StubHostProbe),
        security_logs_uc: Arc::new(StubSecurityLogs),
        security_audit_uc: Arc::new(StubSecurityAudit),
        tls_cert_uc: Arc::new(StubTlsCert),
        geoip_uc: Arc::new(StubGeoIp),
        game_servers_uc: Arc::new(StubGameServers),
        game_templates_uc: Arc::new(StubGameTemplates),
        game_server_repo: Arc::new(StubGameServerRepo),
        game_template_repo: Arc::new(StubGameTemplateRepo),
        game_audit_repo: Arc::new(StubGameAuditRepo),
        game_session_repo: Arc::new(StubGameSessionRepo),
        game_container_runtime: Arc::new(StubContainerRuntime),
        game_rcon_client: Arc::new(StubRconClient),
        game_port_allocator: Arc::new(StubPortAllocator),
        bump_uc: Arc::new(StubBump),
        eligibility_uc: Arc::new(StubEligibility),
        manage_sponsorships_uc: Arc::new(StubSponsorships),
        dataset_uc: Arc::new(StubDataset),
        ai_jobs_uc: Arc::new(StubAiJobs),
        monthly_ranking_uc: Arc::new(StubMonthlyRanking),
        invitations_uc: Arc::new(StubInvitations),
        oauth_uc: Arc::new(StubOAuth),
        quarantine_uc: Arc::new(StubQuarantine),
        lockdown_uc: Arc::new(StubLockdown),
        slowmode_uc: Arc::new(StubSlowmode),
        component_visibility_uc: Arc::new(StubComponentVisibility),
        component_min_role_uc: Arc::new(StubComponentMinRole),
        alert_rules_uc: Arc::new(StubAlertRules),
        docker_host: Arc::new(StubDockerHost),
        bot_persistence_uc: Arc::new(StubBotPersistence),
        server_events_uc: Arc::new(StubServerEvents),
        rbac_admin_uc: Arc::new(StubRbac),
        sursis_uc: Arc::new(StubSursis),
        automod_adaptive_slowmode_repo: Arc::new(StubAdaptiveSlowmodeRepo),
        game_template_settings_repo: Arc::new(StubGameTemplateSettingsRepo),
        game_session_reg_repo: Arc::new(StubGameSessionRegRepo),
        broadcaster: Arc::new(EventBroadcaster::new()),
        job_client: JobClient::new(redis_client.clone(), "test:jobs".into()),
        discord_api: Arc::new(DiscordApiService::new(String::new())),
        inference: Arc::new(sentinel_api::adapters::outbound::inference_service::InferenceService::new(None, None)),
        api_key: String::new(),
        discord_bot_token: String::new(),
        system_probe: Arc::new(StubSystemProbe),
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
        rbac_global_gate: false,
        rbac_global_gate_audit: false,
        metrics_token: String::new(),
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
pub fn build_test_state_watched_users(
    watched_users_uc: Arc<dyn ManageWatchedUsersUseCase>,
) -> AppState {
    let mut state = base_state();
    state.watched_users_uc = watched_users_uc;
    state
}

/// Construit un AppState avec un mock user activity repository injecte.
#[allow(dead_code)]
pub fn build_test_state_user_activity(
    user_activity_repo: Arc<dyn UserActivityRepository>,
) -> AppState {
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
pub fn build_test_state_daily_activity(
    daily_activity_repo: Arc<dyn DailyActivityRepository>,
) -> AppState {
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
    pub fn new() -> Self {
        Self::default()
    }
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
    async fn list_all_channels(&self, _: &str) -> Result<Vec<DiscordChannel>, DomainError> {
        self.record("list_all_channels");
        Ok(vec![])
    }
    async fn upload_emoji(
        &self,
        _: &str,
        _: &str,
        _: &[u8],
        _: &str,
    ) -> Result<(String, String, bool), DomainError> {
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
    async fn create_role(
        &self,
        _: &str,
        _: &str,
        _: u32,
        _: Option<&str>,
    ) -> Result<serde_json::Value, DomainError> {
        self.record("create_role");
        Ok(serde_json::json!({"id": "r1", "name": "role"}))
    }
    async fn edit_role(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: Option<u32>,
        _: Option<&str>,
        _: Option<bool>,
        _: Option<bool>,
    ) -> Result<serde_json::Value, DomainError> {
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
        Ok(DiscordUser {
            id: "u1".into(),
            username: "mock".into(),
            avatar: None,
        })
    }
}

// ── Stub Voice Channels (needed for base_state) ──

pub struct StubVoiceChannels;
#[async_trait]
impl ManageVoiceChannelsUseCase for StubVoiceChannels {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError> {
        unimplemented!()
    }
    async fn list_channels(&self, _: &str) -> Result<Vec<VoiceChannel>, DomainError> {
        unimplemented!()
    }
    async fn list_history_channels(
        &self,
        _: &str,
        _: i64,
    ) -> Result<Vec<VoiceChannel>, DomainError> {
        unimplemented!()
    }
    async fn get_voice_config(&self, _: &str) -> Result<VoiceChannelConfig, DomainError> {
        Ok(VoiceChannelConfig::default())
    }
    async fn get_channel_detail(&self, _: &str) -> Result<VoiceChannelDetail, DomainError> {
        unimplemented!()
    }
    async fn create_channel(
        &self,
        _: CreateVoiceChannelCommand,
    ) -> Result<VoiceChannel, DomainError> {
        unimplemented!()
    }
    async fn close_channel(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn delete_channel(&self, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn find_guild_id(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn purge_channel(&self, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn purge_history(&self, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn update_channel(&self, _: UpdateVoiceChannelCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn transfer_ownership(&self, _: TransferOwnershipCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn add_co_admin(&self, _: ManageCoAdminCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn remove_co_admin(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get_whitelist(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> {
        unimplemented!()
    }
    async fn add_to_whitelist(&self, _: ManageWhitelistCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn remove_from_whitelist(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get_preset(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Option<sentinel_core::domain::entities::community::voice_channel::VoiceChannelPreset>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn save_preset(
        &self,
        _: sentinel_core::ports::inbound::community::manage_voice_channels::SavePresetCommand,
    ) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn ban_from_channel(&self, _: BanFromChannelCommand) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn unban_from_channel(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn is_banned(&self, _: &str, _: &str) -> Result<bool, DomainError> {
        unimplemented!()
    }
    async fn list_owner_bans(
        &self,
        _: &str,
        _: &str,
    ) -> Result<
        Vec<sentinel_core::domain::entities::community::voice_channel::VoiceChannelBan>,
        DomainError,
    > {
        unimplemented!()
    }
    async fn create_invite_link(
        &self,
        _: CreateInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        unimplemented!()
    }
    async fn list_invite_links(&self, _: &str) -> Result<Vec<VoiceChannelInviteLink>, DomainError> {
        unimplemented!()
    }
    async fn use_invite_link(
        &self,
        _: UseInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        unimplemented!()
    }
    async fn revoke_invite_link(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn list_themes(&self, _: &str) -> Result<Vec<VoiceChannelTheme>, DomainError> {
        unimplemented!()
    }
    async fn create_theme(&self, _: CreateThemeCommand) -> Result<VoiceChannelTheme, DomainError> {
        unimplemented!()
    }
    async fn update_theme(
        &self,
        _: &str,
        _: CreateThemeCommand,
    ) -> Result<VoiceChannelTheme, DomainError> {
        unimplemented!()
    }
    async fn delete_theme(&self, _: &str, _: &str) -> Result<(), DomainError> {
        unimplemented!()
    }
}

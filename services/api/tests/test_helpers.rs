//! Test helpers : construit un AppState complet avec des stubs pour tous les traits.
//! Seul le use case sous test est fonctionnel, les autres panic si appeles.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_api::adapters::inbound::ws::broadcaster::EventBroadcaster;
use sentinel_api::adapters::outbound::job_client::JobClient;
use sentinel_api::domain::entities::*;
use sentinel_api::domain::entities::analytics::*;
use sentinel_api::domain::errors::DomainError;
use sentinel_api::domain::services::DiscordApiService;
use sentinel_api::ports::inbound::*;
use sentinel_api::ports::outbound::*;

// ══════════════════════════════════════════════════════════
// Macro pour generer des stub traits (panic si appeles)
// ══════════════════════════════════════════════════════════

macro_rules! stub_async_method {
    ($method:ident ( $($arg:ident : $ty:ty),* ) -> $ret:ty) => {
        async fn $method(&self, $($arg: $ty),*) -> $ret {
            unimplemented!(concat!("Stub: ", stringify!($method), " not implemented for tests"))
        }
    };
}

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
}

pub struct StubSecurity;
#[async_trait]
impl ManageSecurityUseCase for StubSecurity {
    async fn report_event(&self, _: ReportSecurityEventCommand) -> Result<SecurityEvent, DomainError> { unimplemented!() }
    async fn list_events(&self, _: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> { unimplemented!() }
}

pub struct StubModeration;
#[async_trait]
impl ManageModerationUseCase for StubModeration {
    async fn log_action(&self, _: LogModerationCommand) -> Result<ModerationAction, DomainError> { unimplemented!() }
    async fn get_history(&self, _: &str, _: &str) -> Result<UserModerationHistory, DomainError> { unimplemented!() }
    async fn list_bans(&self, _: Option<&str>, _: i64, _: i64) -> Result<Vec<ModerationAction>, DomainError> { unimplemented!() }
    async fn delete_bans_for_user(&self, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
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

pub struct StubConduct;
#[async_trait]
impl ManageConductUseCase for StubConduct {
    async fn get_config(&self, _: &str) -> Result<ConductConfig, DomainError> { unimplemented!() }
    async fn save_config(&self, _: SaveConductConfigCommand) -> Result<ConductConfig, DomainError> { unimplemented!() }
    async fn get_points(&self, _: &str, _: &str) -> Result<UserConductPoints, DomainError> { unimplemented!() }
    async fn deduct_points(&self, _: DeductPointsCommand) -> Result<UserConductPoints, DomainError> { unimplemented!() }
    async fn add_points(&self, _: AddPointsCommand) -> Result<UserConductPoints, DomainError> { unimplemented!() }
    async fn get_leaderboard(&self, _: &str, _: i64) -> Result<Vec<UserConductPoints>, DomainError> { unimplemented!() }
    async fn get_points_log(&self, _: &str, _: &str, _: i64) -> Result<Vec<ConductPointsLog>, DomainError> { unimplemented!() }
    async fn run_regen(&self) -> Result<u64, DomainError> { unimplemented!() }
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
    async fn create(&self, _: manage_audit_logs::CreateAuditLogCommand) -> Result<AuditLog, DomainError> { unimplemented!() }
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
    async fn get_rewards(&self, _: &str) -> Result<Vec<LevelReward>, DomainError> { unimplemented!() }
    async fn get_rewards_by_source(&self, _: &str, _: XpSource) -> Result<Vec<LevelReward>, DomainError> { unimplemented!() }
    async fn set_reward(&self, _: &str, _: i32, _: &str, _: XpSource) -> Result<LevelReward, DomainError> { unimplemented!() }
    async fn delete_reward(&self, _: &str, _: i32, _: XpSource) -> Result<(), DomainError> { unimplemented!() }
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
}

pub struct StubBotConfigRepo;
#[async_trait]
impl BotConfigRepository for StubBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> { unimplemented!() }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> { unimplemented!() }
    async fn get_all_config(&self, _: &str) -> Result<Vec<BotGuildConfig>, DomainError> { unimplemented!() }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> { unimplemented!() }
}

pub struct StubIaConfigRepo;
#[async_trait]
impl IaConfigRepository for StubIaConfigRepo {
    async fn get(&self, _: &str) -> Result<Option<IaConfig>, DomainError> { unimplemented!() }
    async fn save(&self, _: &IaConfig) -> Result<IaConfig, DomainError> { unimplemented!() }
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
    async fn leaderboard(&self, _: &str, _: i64) -> Result<Vec<Wallet>, DomainError> { unimplemented!() }
    async fn get_transactions(&self, _: &str, _: &str, _: i64) -> Result<Vec<WalletTransaction>, DomainError> { unimplemented!() }
}

pub struct StubBlackjackRepo;
#[async_trait]
impl BlackjackRepository for StubBlackjackRepo {
    async fn create(&self, _: &BlackjackGame) -> Result<(), DomainError> { unimplemented!() }
    async fn get_active(&self, _: &str, _: &str) -> Result<Option<BlackjackGame>, DomainError> { unimplemented!() }
    async fn update(&self, _: &BlackjackGame) -> Result<(), DomainError> { unimplemented!() }
    async fn get_by_id(&self, _: Uuid) -> Result<Option<BlackjackGame>, DomainError> { unimplemented!() }
}

// ══════════════════════════════════════════════════════════
// TestAppState builder
// ══════════════════════════════════════════════════════════

/// Construit un AppState de base avec tous les stubs.
fn base_state() -> AppState {
    let redis_client = redis::Client::open("redis://localhost:6379").unwrap();
    let pg_pool = sqlx::PgPool::connect_lazy("postgres://localhost/fake_test_db").unwrap();

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
        conduct_uc: Arc::new(StubConduct),
        watched_users_uc: Arc::new(StubWatchedUsers),
        audit_logs_uc: Arc::new(StubAuditLogs),
        levels_uc: Arc::new(StubLevels),
        role_panels_uc: Arc::new(StubRolePanels),
        notes_uc: Arc::new(StubNotes),
        reminders_uc: Arc::new(StubReminders),
        strikes_uc: Arc::new(StubStrikes),
        analytics_repo: Arc::new(StubAnalyticsRepo),
        daily_activity_repo: Arc::new(StubDailyActivityRepo),
        log_repo: Arc::new(StubLogRepo),
        guild_repo: Arc::new(StubGuildRepo),
        bot_config_repo: Arc::new(StubBotConfigRepo),
        ia_config_repo: Arc::new(StubIaConfigRepo),
        discord_role_repo: Arc::new(StubDiscordRoleRepo),
        members_uc: Arc::new(StubMembers),
        wallet_repo: Arc::new(StubWalletRepo),
        blackjack_svc: Arc::new(sentinel_api::application::BlackjackService::new(
            Arc::new(StubBlackjackRepo),
            Arc::new(StubWalletRepo),
        )),
        broadcaster: Arc::new(EventBroadcaster::new()),
        job_client: JobClient::new(redis_client.clone(), "test:jobs".into()),
        discord_api: Arc::new(DiscordApiService::new(String::new())),
        inference: Arc::new(sentinel_api::domain::services::InferenceService::new(None, None)),
        api_key: String::new(),
        discord_bot_token: String::new(),
        pg_pool,
        redis_client,
        cache: None,
    }
}

/// Construit un AppState avec un mock voice channels injecte.
pub fn build_test_state(voice_uc: Arc<dyn ManageVoiceChannelsUseCase>) -> AppState {
    let mut state = base_state();
    state.voice_channels_uc = voice_uc;
    state
}

/// Construit un AppState avec un mock tickets injecte.
pub fn build_test_state_tickets(tickets_uc: Arc<dyn ManageTicketsUseCase>) -> AppState {
    let mut state = base_state();
    state.tickets_uc = tickets_uc;
    state
}

/// Construit un AppState avec un mock strikes injecte.
pub fn build_test_state_strikes(strikes_uc: Arc<dyn ManageStrikesUseCase>) -> AppState {
    let mut state = base_state();
    state.strikes_uc = strikes_uc;
    state
}

// ── Stub Voice Channels (needed for base_state) ──

pub struct StubVoiceChannels;
#[async_trait]
impl ManageVoiceChannelsUseCase for StubVoiceChannels {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError> { unimplemented!() }
    async fn list_channels(&self, _: &str) -> Result<Vec<VoiceChannel>, DomainError> { unimplemented!() }
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

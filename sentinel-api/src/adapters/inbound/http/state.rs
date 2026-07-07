use std::sync::Arc;

use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::discord_api::DiscordApi;
use crate::adapters::outbound::inference_service::InferenceService;
use crate::adapters::outbound::job_client::JobClient;
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::application::casino::blackjack_service::BlackjackService;
use crate::ports::inbound::ai::analyze_image::AnalyzeImageUseCase;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use crate::ports::inbound::ai::manage_dataset::ManageDatasetUseCase;
use crate::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use crate::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use crate::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use crate::ports::inbound::audit::manage_watched_users::ManageWatchedUsersUseCase;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::community::manage_bump::ManageBumpUseCase;
use crate::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use crate::ports::inbound::community::manage_members::ManageMembersUseCase;
use crate::ports::inbound::community::manage_role_panels::ManageRolePanelsUseCase;
use crate::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase;
use crate::ports::inbound::coude::manage_economy::ManageCoudeEconomyUseCase;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use crate::ports::inbound::game::manage_game_servers::ManageGameServersUseCase;
use crate::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase;
use crate::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use crate::ports::inbound::moderation::manage_notes::ManageNotesUseCase;
use crate::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase;
use crate::ports::inbound::moderation::manage_rules::ManageRulesUseCase;
use crate::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase;
use crate::ports::inbound::system::manage_tickets::ManageTicketsUseCase;
use crate::ports::outbound::audit::analytics_repository::AnalyticsRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::outbound::community::daily_activity_repository::DailyActivityRepository;
use crate::ports::outbound::community::discord_role_repository::DiscordRoleRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::system::guild_repository::GuildRepository;
use crate::ports::outbound::system::log_repository::LogRepository;
#[derive(Clone)]
pub struct AppState {
    pub analyze_uc: Arc<dyn AnalyzeMessageUseCase>,
    pub analyze_image_uc: Arc<dyn AnalyzeImageUseCase>,
    pub dataset_uc: Arc<dyn ManageDatasetUseCase>,
    pub ai_jobs_uc: Arc<dyn crate::ports::inbound::ai::manage_ai_jobs::ManageAiJobsUseCase>,
    pub rules_uc: Arc<dyn ManageRulesUseCase>,
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    pub tickets_uc: Arc<dyn ManageTicketsUseCase>,
    pub invitations_uc: Arc<dyn crate::ports::inbound::system::manage_invitations::ManageInvitationsUseCase>,
    pub security_uc: Arc<dyn ManageSecurityUseCase>,
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    pub modstats_uc: Arc<dyn crate::ports::inbound::moderation::read_modstats::ReadModstatsUseCase>,
    pub stats_uc: Arc<dyn ManageStatsUseCase>,
    pub voice_channels_uc: Arc<dyn ManageVoiceChannelsUseCase>,
    pub watched_users_uc: Arc<dyn ManageWatchedUsersUseCase>,
    pub audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>,
    pub snapshots_uc: Arc<dyn crate::ports::inbound::audit::manage_snapshots::ManageSnapshotsUseCase>,
    pub levels_uc: Arc<dyn ManageLevelsUseCase>,
    pub announcements_uc: Arc<dyn crate::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase>,
    pub confessions_uc: Arc<dyn crate::ports::inbound::community::manage_confessions::ManageConfessionsUseCase>,
    pub role_panels_uc: Arc<dyn ManageRolePanelsUseCase>,
    pub notes_uc: Arc<dyn ManageNotesUseCase>,
    pub bump_uc: Arc<dyn ManageBumpUseCase>,
    pub monthly_ranking_uc:
        Arc<dyn crate::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase>,
    pub reminders_uc: Arc<dyn ManageRemindersUseCase>,
    pub strikes_uc: Arc<dyn ManageStrikesUseCase>,
    pub moderation_copilot_uc:
        Arc<dyn crate::ports::inbound::moderation::moderation_copilot::ModerationCopilotUseCase>,
    pub members_uc: Arc<dyn ManageMembersUseCase>,
    pub analytics_repo: Arc<dyn AnalyticsRepository>,
    pub daily_activity_repo: Arc<dyn DailyActivityRepository>,
    pub age_ban_repo: Arc<dyn crate::ports::outbound::community::age_ban_repository::AgeBanRepository>,
    pub log_repo: Arc<dyn LogRepository>,
    pub system_logs_uc:
        Arc<dyn crate::ports::inbound::system::manage_system_logs::ManageSystemLogsUseCase>,
    pub guild_repo: Arc<dyn GuildRepository>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    pub discord_role_repo: Arc<dyn DiscordRoleRepository>,
    pub wallet_repo: Arc<dyn WalletRepository>,
    pub wallet_uc: Arc<dyn ManageWalletUseCase>,
    pub blackjack_svc: Arc<BlackjackService>,
    pub slot_uc: Arc<dyn crate::ports::inbound::casino::manage_slot::ManageSlotUseCase>,
    pub wheel_uc: Arc<dyn crate::ports::inbound::casino::manage_wheel::ManageWheelUseCase>,
    pub coude_players_uc: Arc<dyn ManageCoudePlayersUseCase>,
    pub coude_combats_uc: Arc<dyn ManageCoudeCombatsUseCase>,
    pub coude_bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
    pub coude_economy_uc: Arc<dyn ManageCoudeEconomyUseCase>,
    pub coude_inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
    pub coude_social_uc: Arc<dyn ManageCoudeSocialUseCase>,
    pub resolve_betting_batch_uc: Arc<dyn crate::ports::inbound::coude::resolve_betting_batch::ResolveBettingBatchUseCase>,
    pub expire_combats_batch_uc: Arc<dyn crate::ports::inbound::coude::expire_combats_batch::ExpireCombatsBatchUseCase>,
    pub resolve_combat_now_uc: Arc<dyn crate::ports::inbound::coude::resolve_combat_now::ResolveCombatNowUseCase>,
    pub resolve_friendly_duel_uc: Arc<dyn crate::ports::inbound::coude::resolve_friendly_duel::ResolveFriendlyDuelUseCase>,
    pub coude_catalog_uc: Arc<dyn crate::ports::inbound::coude::manage_catalog::ManageCoudeCatalogUseCase>,
    pub coude_cashbox_uc: Arc<dyn crate::ports::inbound::coude::manage_cashbox::ManageCoudeCashboxUseCase>,
    pub coude_steal_protections_uc:
        Arc<dyn crate::ports::inbound::coude::manage_steal_protections::ManageCoudeStealProtectionsUseCase>,
    pub coude_steal_boosts_uc: Arc<dyn crate::ports::inbound::coude::manage_steal_boosts::ManageCoudeStealBoostsUseCase>,
    pub coude_steal_attempts_uc:
        Arc<dyn crate::ports::inbound::coude::manage_steal_attempts::ManageStealAttemptsUseCase>,
    pub coude_taunts_uc: Arc<dyn crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase>,
    pub coude_heist_uc: Arc<dyn crate::ports::inbound::coude::manage_heist::ManageCoudeHeistUseCase>,
    pub coude_curses_uc: Arc<dyn crate::ports::inbound::coude::manage_curses::ManageCoudeCursesUseCase>,
    pub coude_safety_net_uc: Arc<dyn crate::ports::inbound::coude::manage_safety_net::ManageCoudeSafetyNetUseCase>,
    pub tournaments_uc: Arc<dyn crate::ports::inbound::coude::manage_tournaments::ManageTournamentsUseCase>,
    pub coude_tout_ou_rien_repo: Arc<dyn crate::ports::outbound::coude::tout_ou_rien_repository::ToutOuRienRepository>,
    pub play_tout_ou_rien_uc: Arc<dyn crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienUseCase>,
    pub roll_steal_uc: Arc<dyn crate::ports::inbound::coude::roll_steal::RollStealUseCase>,
    pub coude_flavor_templates_repo: Arc<dyn crate::ports::outbound::coude::flavor_templates_repository::FlavorTemplatesRepository>,
    pub discord_action_messages_uc:
        Arc<dyn crate::ports::inbound::audit::manage_discord_action_messages::ManageDiscordActionMessagesUseCase>,
    pub coude_refusal_count_repo: Arc<dyn crate::ports::outbound::coude::refusal_count_repository::RefusalCountRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
    #[allow(dead_code)]
    pub job_client: JobClient,
    pub discord_api: Arc<dyn DiscordApi>,
    pub inference: Arc<InferenceService>,
    pub api_key: String,
    #[allow(dead_code)]
    pub discord_bot_token: String,
    pub user_activity_repo: Arc<dyn crate::ports::outbound::audit::user_activity_repository::UserActivityRepository>,
    pub welcome_config_uc: Arc<dyn crate::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase>,
    pub automod_reviews_uc: Arc<dyn crate::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase>,
    pub automod_adaptive_slowmode_repo: Arc<dyn crate::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository>,
    pub reset_guild_uc: Arc<dyn crate::ports::inbound::system::reset_guild::ResetGuildUseCase>,
    pub pets_uc: Arc<dyn crate::ports::inbound::tamagotchi::manage_pets::ManagePetsUseCase>,
    pub rotation_uc: Arc<dyn crate::ports::inbound::system::manage_rotation::ManageRotationUseCase>,
    pub ip_bans_uc: Arc<dyn crate::ports::inbound::system::manage_ip_bans::ManageIpBansUseCase>,
    pub host_probe_uc: Arc<dyn crate::ports::inbound::system::read_host_probe::ReadHostProbeUseCase>,
    pub security_logs_uc: Arc<dyn crate::ports::inbound::system::read_security_logs::ReadSecurityLogsUseCase>,
    pub security_audit_uc: Arc<dyn crate::ports::inbound::system::manage_security_audit::ManageSecurityAuditUseCase>,
    pub oauth_uc: Arc<dyn crate::ports::inbound::system::manage_oauth::ManageOAuthUseCase>,
    pub quarantine_uc:
        Arc<dyn crate::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase>,
    pub lockdown_uc:
        Arc<dyn crate::ports::inbound::system::manage_lockdown::ManageLockdownUseCase>,
    pub slowmode_uc:
        Arc<dyn crate::ports::inbound::system::manage_slowmode::ManageSlowmodeUseCase>,
    pub component_visibility_uc: Arc<
        dyn crate::ports::inbound::system::manage_component_visibility::ManageComponentVisibilityUseCase,
    >,
    pub component_min_role_uc: Arc<
        dyn crate::ports::inbound::system::manage_component_min_role::ManageComponentMinRoleUseCase,
    >,
    pub bot_persistence_uc:
        Arc<dyn crate::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase>,
    pub server_events_uc:
        Arc<dyn crate::ports::inbound::system::manage_server_events::ManageServerEventsUseCase>,
    /// CRUD RBAC applicatif (endpoints owner). Nomme `rbac_admin_uc` pour ne pas
    /// confondre avec le middleware RBAC (`middleware/rbac.rs`), qui a sa propre
    /// logique de resolution de role.
    pub rbac_admin_uc: Arc<dyn crate::ports::inbound::system::manage_rbac::ManageRbacUseCase>,
    pub tls_cert_uc: Arc<dyn crate::ports::inbound::system::read_tls_cert::ReadTlsCertUseCase>,
    pub geoip_uc: Arc<dyn crate::ports::inbound::system::lookup_geoip::LookupGeoIpUseCase>,
    pub export_uc: Arc<dyn crate::application::system::export_service::ExecuteExportUseCase>,
    pub export_jobs_uc:
        Arc<dyn crate::ports::inbound::system::manage_export_jobs::ManageExportJobsUseCase>,
    pub evidence_repo: Arc<dyn crate::ports::outbound::moderation::evidence_repository::EvidenceRepository>,
    pub review_repo: Arc<dyn crate::ports::outbound::moderation::review_repository::ReviewRepository>,
    pub modstats_repo: Arc<dyn crate::ports::outbound::audit::modstats_repository::ModstatsRepository>,
    pub game_repo: Arc<dyn crate::ports::outbound::casino::game_repository::GameRepository>,
    pub sponsorship_repo: Arc<dyn crate::ports::outbound::coude::sponsorship_repository::SponsorshipRepository>,
    pub temp_role_repo: Arc<dyn crate::ports::outbound::community::temp_role_repository::TempRoleRepository>,
    /// Use case Community (sponsorships + temp-roles) derriere le service gRPC.
    pub manage_sponsorships_uc: Arc<dyn crate::ports::inbound::community::manage_sponsorships::ManageSponsorshipsUseCase>,
    pub pending_action_repo: Arc<dyn crate::ports::outbound::moderation::pending_action_repository::PendingActionRepository>,
    pub blackjack_table_repo: Arc<dyn crate::ports::outbound::casino::blackjack_table_repository::BlackjackTableRepository>,
    /// Game Portal : use cases lifecycle serveurs Docker.
    pub game_servers_uc: Arc<dyn ManageGameServersUseCase>,
    pub game_templates_uc: Arc<dyn ManageGameTemplatesUseCase>,
    /// Game Portal : adapters exposes pour les endpoints internes /jobs/*
    /// appeles par game-portal-worker (health/idle/reconcile).
    pub game_server_repo: Arc<dyn crate::ports::outbound::game::game_server_repository::GameServerRepository>,
    pub game_template_repo: Arc<dyn crate::ports::outbound::game::game_template_repository::GameTemplateRepository>,
    pub game_template_settings_repo: Arc<dyn crate::ports::outbound::game::game_session_repository::GameTemplateSettingsRepository>,
    pub game_session_reg_repo: Arc<dyn crate::ports::outbound::game::game_session_repository::GameSessionRegistrationRepository>,
    pub game_audit_repo: Arc<dyn crate::ports::outbound::game::game_audit_repository::GameAuditRepository>,
    pub game_session_repo: Arc<dyn crate::ports::outbound::game::player_session_repository::PlayerSessionRepository>,
    pub game_container_runtime: Arc<dyn crate::ports::outbound::game::container_runtime::ContainerRuntime>,
    pub game_rcon_client: Arc<dyn crate::ports::outbound::game::rcon_client::RconClient>,
    pub game_port_allocator: Arc<dyn crate::ports::outbound::game::port_allocator::PortAllocator>,
    /// Jeu Influence — use cases.
    pub influence_view_profile_uc:
        Arc<dyn crate::ports::inbound::influence::view_profile::ViewProfileUseCase>,
    pub influence_orgs_uc: Arc<
        dyn crate::ports::inbound::influence::manage_organizations::ManageOrganizationsUseCase,
    >,
    pub influence_votes_uc:
        Arc<dyn crate::ports::inbound::influence::manage_votes::ManageVotesUseCase>,
    pub influence_capital_uc:
        Arc<dyn crate::ports::inbound::influence::manage_capital::ManageCapitalUseCase>,
    pub influence_laws_uc:
        Arc<dyn crate::ports::inbound::influence::manage_laws::ManageLawsUseCase>,
    pub influence_information_uc:
        Arc<dyn crate::ports::inbound::influence::manage_information::ManageInformationUseCase>,
    pub influence_archives_uc:
        Arc<dyn crate::ports::inbound::influence::read_archives::ReadArchivesUseCase>,
    pub sursis_uc:
        Arc<dyn crate::ports::inbound::moderation::manage_sursis::ManageSursisUseCase>,
    pub pg_pool: sqlx::PgPool,
    pub redis_client: redis::Client,
    pub cache: Option<Arc<RedisCache>>,
    /// Phase 7 B — Liste des Discord user_ids superadmin (env SUPERADMIN_USER_IDS).
    /// Utilisee pour gater les endpoints globaux non scoped par guild (ex: /purge/logs).
    pub superadmin_user_ids: Arc<Vec<String>>,
    /// OAuth Discord — credentials cote serveur (jamais exposes au front).
    pub discord_oauth_client_id: String,
    pub discord_oauth_client_secret: String,
    pub discord_oauth_redirect_uri: String,
    pub web_front_url: String,
    /// Container monitor : poll Docker chaque minute, detecte les changements.
    pub container_monitor: Option<std::sync::Arc<tokio::sync::RwLock<crate::adapters::outbound::system::container_monitor::ContainerMonitorState>>>,
    /// Rate limiter dynamique : tracking req/IP en memoire pour ban auto.
    pub rate_limiter: Option<std::sync::Arc<crate::adapters::outbound::system::rate_limiter::RateLimiter>>,
    /// Feature flag — active le `global_rbac_gate` (gate RBAC global
    /// fail-closed sur les mutations web). Default `false` = no-op.
    /// Voir `middleware/global_rbac.rs`.
    pub rbac_global_gate: bool,
}

impl AppState {
    /// Lit le delai de rappel avant expiration depuis la config guild
    /// (cle `reminder_advance_secs` du bot `moderation-bot`). Default 3600s = 1h.
    pub async fn bot_config_reminder_advance_secs(&self, guild_id: &str) -> u64 {
        match self
            .bot_config_repo
            .get_config(guild_id, "moderation-bot")
            .await
        {
            Ok(entries) => entries
                .iter()
                .find(|e| e.config_key == "reminder_advance_secs")
                .and_then(|e| e.config_value.parse().ok())
                .unwrap_or(3600),
            Err(_) => 3600,
        }
    }
}

use std::sync::Arc;

use crate::adapters::outbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::discord_api::DiscordApi;
use crate::adapters::outbound::inference_service::InferenceService;
use crate::adapters::outbound::job_client::JobClient;
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::ports::inbound::ai::analyze_image::AnalyzeImageUseCase;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use crate::ports::inbound::ai::manage_dataset::ManageDatasetUseCase;
use crate::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use crate::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use crate::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use crate::ports::inbound::audit::manage_watched_users::ManageWatchedUsersUseCase;
use crate::ports::inbound::community::manage_bump::ManageBumpUseCase;
use crate::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use crate::ports::inbound::community::manage_members::ManageMembersUseCase;
use crate::ports::inbound::community::manage_role_panels::ManageRolePanelsUseCase;
use crate::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase;
use crate::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use crate::ports::inbound::moderation::manage_notes::ManageNotesUseCase;
use crate::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase;
use crate::ports::inbound::moderation::manage_rules::ManageRulesUseCase;
use crate::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase;
use crate::ports::inbound::system::manage_tickets::ManageTicketsUseCase;
use crate::ports::outbound::audit::analytics_repository::AnalyticsRepository;
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
    pub security_uc: Arc<dyn ManageSecurityUseCase>,
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    pub modstats_uc: Arc<dyn crate::ports::inbound::moderation::read_modstats::ReadModstatsUseCase>,
    pub stats_uc: Arc<dyn ManageStatsUseCase>,
    pub voice_channels_uc: Arc<dyn ManageVoiceChannelsUseCase>,
    pub watched_users_uc: Arc<dyn ManageWatchedUsersUseCase>,
    pub audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>,
    /// Planning communautaire (evenements et campagnes de jeu).
    pub events_uc:
        Arc<dyn sentinel_core::ports::inbound::community::manage_events::ManageEventsUseCase>,
    /// Vie de la communaute affichee dans l'espace membre.
    pub lfg_uc:
        Arc<dyn sentinel_core::ports::inbound::community::manage_lfg::ManageLfgUseCase>,
    pub polls_uc:
        Arc<dyn sentinel_core::ports::inbound::community::manage_polls::ManagePollsUseCase>,
    pub spotlight_uc: Arc<
        dyn sentinel_core::ports::inbound::community::manage_spotlight::ManageSpotlightUseCase,
    >,
    pub news_uc:
        Arc<dyn sentinel_core::ports::inbound::community::manage_news::ManageNewsUseCase>,
    /// Presence en direct, publiee par le bot dans Redis.
    pub presence_uc: Arc<
        dyn sentinel_core::ports::inbound::community::read_presence::ReadPresenceUseCase,
    >,
    pub detect_anomaly_uc: Arc<dyn crate::ports::inbound::audit::detect_moderation_anomaly::DetectModerationAnomalyUseCase>,
    pub weekly_report_uc: Arc<dyn crate::ports::inbound::audit::get_weekly_report::GetWeeklyReportUseCase>,
    pub snapshots_uc: Arc<dyn crate::ports::inbound::audit::manage_snapshots::ManageSnapshotsUseCase>,
    pub levels_uc: Arc<dyn ManageLevelsUseCase>,
    pub announcements_uc: Arc<dyn crate::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase>,
    pub embeds_uc: Arc<dyn crate::ports::inbound::community::manage_embeds::ManageEmbedsUseCase>,
    pub confessions_uc: Arc<dyn crate::ports::inbound::community::manage_confessions::ManageConfessionsUseCase>,
    pub role_panels_uc: Arc<dyn ManageRolePanelsUseCase>,
    pub notes_uc: Arc<dyn ManageNotesUseCase>,
    pub bump_uc: Arc<dyn ManageBumpUseCase>,
    pub eligibility_uc:
        Arc<dyn crate::ports::inbound::community::check_eligibility::CheckEligibilityUseCase>,
    pub monthly_ranking_uc:
        Arc<dyn crate::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase>,
    pub reminders_uc: Arc<dyn ManageRemindersUseCase>,
    pub strikes_uc: Arc<dyn ManageStrikesUseCase>,
    pub moderation_copilot_uc:
        Arc<dyn crate::ports::inbound::moderation::moderation_copilot::ModerationCopilotUseCase>,
    /// Evaluation server-side du risque d'une cible (seuil + politique de
    /// confirmation). Le bot fournit les faits Discord, l'API decide.
    pub assess_target_risk_uc:
        Arc<dyn crate::ports::inbound::moderation::assess_target_risk::AssessTargetRiskUseCase>,
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
    pub discord_action_messages_uc:
        Arc<dyn crate::ports::inbound::audit::manage_discord_action_messages::ManageDiscordActionMessagesUseCase>,
    pub broadcaster: Arc<EventBroadcaster>,
    #[allow(dead_code)]
    pub job_client: JobClient,
    pub discord_api: Arc<dyn DiscordApi>,
    pub inference: Arc<InferenceService>,
    pub api_key: String,
    /// Serveur Discord unique servi par cette installation. Vide =
    /// verrou desactive (cf. `middleware::single_guild`).
    pub guild_id: String,
    /// Relais vers la plateforme jeux. Seul chemin d'acces aux jeux
    /// depuis le web : le navigateur ne joint jamais nexus-api.
    pub nexus_games: Arc<crate::adapters::outbound::nexus_games::NexusGamesClient>,
    /// Token optionnel protégeant `/metrics` (vide = ouvert). Voir config.
    pub metrics_token: String,
    #[allow(dead_code)]
    pub discord_bot_token: String,
    pub user_activity_repo: Arc<dyn crate::ports::outbound::audit::user_activity_repository::UserActivityRepository>,
    pub welcome_config_uc: Arc<dyn crate::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase>,
    /// Verification d'age : decision server-side (seuil pass/ban + duree de ban).
    pub age_check_uc: Arc<dyn crate::ports::inbound::community::evaluate_age_declaration::EvaluateAgeDeclarationUseCase>,
    pub automod_reviews_uc: Arc<dyn crate::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase>,
    pub automod_adaptive_slowmode_repo: Arc<dyn crate::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository>,
    pub reset_guild_uc: Arc<dyn crate::ports::inbound::system::reset_guild::ResetGuildUseCase>,
    /// Sauvegarde / restauration de serveur (domaine `guild_backup`).
    pub guild_snapshots_uc: Arc<
        dyn crate::ports::inbound::guild_backup::manage_snapshots::ManageGuildSnapshotsUseCase,
    >,
    /// Re-attribution des roles aux membres a leur retour (`guild_backup`).
    pub pending_role_grants_uc: Arc<
        dyn crate::ports::inbound::guild_backup::manage_pending_role_grants::ManagePendingRoleGrantsUseCase,
    >,
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
    pub alert_rules_uc:
        Arc<dyn crate::ports::inbound::system::manage_alert_rules::ManageAlertRulesUseCase>,
    pub bot_persistence_uc:
        Arc<dyn crate::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase>,
    pub server_events_uc:
        Arc<dyn crate::ports::inbound::system::manage_server_events::ManageServerEventsUseCase>,
    pub tls_cert_uc: Arc<dyn crate::ports::inbound::system::read_tls_cert::ReadTlsCertUseCase>,
    /// Acces au daemon Docker de l'hote (listing, actions, prune, df).
    pub docker_host: Arc<dyn crate::ports::outbound::system::docker_host::DockerHost>,
    pub geoip_uc: Arc<dyn crate::ports::inbound::system::lookup_geoip::LookupGeoIpUseCase>,
    pub export_uc: Arc<dyn crate::application::system::export_service::ExecuteExportUseCase>,
    pub export_jobs_uc:
        Arc<dyn crate::ports::inbound::system::manage_export_jobs::ManageExportJobsUseCase>,
    pub evidence_repo: Arc<dyn crate::ports::outbound::moderation::evidence_repository::EvidenceRepository>,
    pub review_repo: Arc<dyn crate::ports::outbound::moderation::review_repository::ReviewRepository>,
    pub modstats_repo: Arc<dyn crate::ports::outbound::audit::modstats_repository::ModstatsRepository>,
    pub sponsorship_repo: Arc<dyn crate::ports::outbound::community::sponsorship_repository::SponsorshipRepository>,
    pub temp_role_repo: Arc<dyn crate::ports::outbound::community::temp_role_repository::TempRoleRepository>,
    /// Use case Community (sponsorships + temp-roles) derriere le service gRPC.
    pub manage_sponsorships_uc: Arc<dyn crate::ports::inbound::community::manage_sponsorships::ManageSponsorshipsUseCase>,
    pub pending_action_repo: Arc<dyn crate::ports::outbound::moderation::pending_action_repository::PendingActionRepository>,
    pub sursis_uc:
        Arc<dyn crate::ports::inbound::moderation::manage_sursis::ManageSursisUseCase>,
    /// Sondes sante systeme (taille/disponibilite BDD) derriere un port —
    /// les handlers health/info passent par ici, jamais par `pg_pool`.
    pub system_probe: Arc<dyn crate::ports::outbound::system::system_probe::SystemProbe>,
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
    pub container_monitor: Option<std::sync::Arc<tokio::sync::RwLock<crate::bootstrap::container_monitor::ContainerMonitorState>>>,
    /// Rate limiter dynamique : tracking req/IP en memoire pour ban auto.
    pub rate_limiter: Option<std::sync::Arc<crate::adapters::outbound::system::rate_limiter::RateLimiter>>,

    // ─────────────────────────────────────────────────────────────────────
    // NE PAS utiliser depuis les handlers — passer par un repository
    // outbound (ports/outbound/*). Ce champ n'existe que pour le bootstrap
    // (construction des repositories Pg*) et les tests d'integration
    // (tests/test_helpers.rs) qui construisent AppState hors du crate.
    // ─────────────────────────────────────────────────────────────────────
    #[doc = "Reserve au bootstrap et aux tests d'integration. Aucun handler \
             inbound ne doit executer de SQL via ce pool : creer/utiliser un \
             port outbound (ex: SystemProbe pour les sondes sante)."]
    pub pg_pool: sqlx::PgPool,
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

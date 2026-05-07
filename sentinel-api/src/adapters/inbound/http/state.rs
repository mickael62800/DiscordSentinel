use std::sync::Arc;

use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::job_client::JobClient;
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::adapters::outbound::discord_api::DiscordApi;
use crate::adapters::outbound::inference_service::InferenceService;
use crate::ports::inbound::ai::analyze_image::AnalyzeImageUseCase;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use crate::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use crate::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use crate::ports::inbound::moderation::manage_notes::ManageNotesUseCase;
use crate::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase;
use crate::ports::inbound::moderation::manage_rules::ManageRulesUseCase;
use crate::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use crate::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use crate::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase;
use crate::ports::inbound::system::manage_tickets::ManageTicketsUseCase;
use crate::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use crate::ports::inbound::community::manage_conduct::ManageConductUseCase;
use crate::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use crate::ports::inbound::community::manage_members::ManageMembersUseCase;
use crate::ports::inbound::community::manage_role_panels::ManageRolePanelsUseCase;
use crate::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase;
use crate::ports::inbound::audit::manage_watched_users::ManageWatchedUsersUseCase;
use crate::ports::inbound::coude::manage_players::ManageCoudePlayersUseCase;
use crate::ports::inbound::casino::manage_wallet::ManageWalletUseCase;
use crate::ports::inbound::coude::manage_combats::ManageCoudeCombatsUseCase;
use crate::ports::inbound::coude::manage_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::coude::manage_economy::ManageCoudeEconomyUseCase;
use crate::ports::inbound::coude::manage_inventory::ManageCoudeInventoryUseCase;
use crate::ports::inbound::coude::manage_social::ManageCoudeSocialUseCase;
use crate::application::casino::blackjack_service::BlackjackService;
use crate::ports::outbound::audit::analytics_repository::AnalyticsRepository;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::community::daily_activity_repository::DailyActivityRepository;
use crate::ports::outbound::community::discord_role_repository::DiscordRoleRepository;
use crate::ports::outbound::system::guild_repository::GuildRepository;
use crate::ports::outbound::system::log_repository::LogRepository;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use crate::ports::inbound::game::manage_game_servers::ManageGameServersUseCase;
use crate::ports::inbound::game::manage_game_templates::ManageGameTemplatesUseCase;
#[derive(Clone)]
pub struct AppState {
    pub analyze_uc: Arc<dyn AnalyzeMessageUseCase>,
    pub analyze_image_uc: Arc<dyn AnalyzeImageUseCase>,
    pub rules_uc: Arc<dyn ManageRulesUseCase>,
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    pub tickets_uc: Arc<dyn ManageTicketsUseCase>,
    pub security_uc: Arc<dyn ManageSecurityUseCase>,
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    pub stats_uc: Arc<dyn ManageStatsUseCase>,
    pub voice_channels_uc: Arc<dyn ManageVoiceChannelsUseCase>,
    pub conduct_uc: Arc<dyn ManageConductUseCase>,
    pub watched_users_uc: Arc<dyn ManageWatchedUsersUseCase>,
    pub audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>,
    pub levels_uc: Arc<dyn ManageLevelsUseCase>,
    pub announcements_uc: Arc<dyn crate::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase>,
    pub confessions_uc: Arc<dyn crate::ports::inbound::community::manage_confessions::ManageConfessionsUseCase>,
    pub role_panels_uc: Arc<dyn ManageRolePanelsUseCase>,
    pub notes_uc: Arc<dyn ManageNotesUseCase>,
    pub reminders_uc: Arc<dyn ManageRemindersUseCase>,
    pub strikes_uc: Arc<dyn ManageStrikesUseCase>,
    pub members_uc: Arc<dyn ManageMembersUseCase>,
    pub analytics_repo: Arc<dyn AnalyticsRepository>,
    pub daily_activity_repo: Arc<dyn DailyActivityRepository>,
    pub log_repo: Arc<dyn LogRepository>,
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
    pub coude_taunts_uc: Arc<dyn crate::ports::inbound::coude::manage_taunts::ManageCoudeTauntsUseCase>,
    pub coude_heist_uc: Arc<dyn crate::ports::inbound::coude::manage_heist::ManageCoudeHeistUseCase>,
    pub coude_curses_uc: Arc<dyn crate::ports::inbound::coude::manage_curses::ManageCoudeCursesUseCase>,
    pub coude_safety_net_uc: Arc<dyn crate::ports::inbound::coude::manage_safety_net::ManageCoudeSafetyNetUseCase>,
    pub coude_vendetta_uc: Arc<dyn crate::ports::inbound::coude::manage_vendetta::ManageCoudeVendettaUseCase>,
    pub coude_tout_ou_rien_repo: Arc<dyn crate::ports::outbound::coude::tout_ou_rien_repository::ToutOuRienRepository>,
    pub play_tout_ou_rien_uc: Arc<dyn crate::ports::inbound::coude::play_tout_ou_rien::PlayToutOuRienUseCase>,
    pub play_travaux_uc: Arc<dyn crate::ports::inbound::coude::play_travaux::PlayTravauxUseCase>,
    pub roll_steal_uc: Arc<dyn crate::ports::inbound::coude::roll_steal::RollStealUseCase>,
    pub coude_flavor_templates_repo: Arc<dyn crate::ports::outbound::coude::flavor_templates_repository::FlavorTemplatesRepository>,
    pub discord_action_messages_uc:
        Arc<dyn crate::ports::inbound::audit::manage_discord_action_messages::ManageDiscordActionMessagesUseCase>,
    pub coude_bounty_repo: Arc<dyn crate::ports::outbound::coude::bounty_repository::BountyRepository>,
    pub coude_refusal_count_repo: Arc<dyn crate::ports::outbound::coude::refusal_count_repository::RefusalCountRepository>,
    pub coude_coalition_repo: Arc<dyn crate::ports::outbound::coude::coalition_repository::CoalitionRepository>,
    pub coude_ultimate_repo: Arc<dyn crate::ports::outbound::coude::ultimate_repository::UltimateRepository>,
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
    pub export_uc: Arc<dyn crate::application::system::export_service::ExecuteExportUseCase>,
    pub evidence_repo: Arc<dyn crate::ports::outbound::moderation::evidence_repository::EvidenceRepository>,
    pub review_repo: Arc<dyn crate::ports::outbound::moderation::review_repository::ReviewRepository>,
    pub modstats_repo: Arc<dyn crate::ports::outbound::audit::modstats_repository::ModstatsRepository>,
    pub game_repo: Arc<dyn crate::ports::outbound::casino::game_repository::GameRepository>,
    pub sponsorship_repo: Arc<dyn crate::ports::outbound::coude::sponsorship_repository::SponsorshipRepository>,
    pub temp_role_repo: Arc<dyn crate::ports::outbound::community::temp_role_repository::TempRoleRepository>,
    pub pending_action_repo: Arc<dyn crate::ports::outbound::moderation::pending_action_repository::PendingActionRepository>,
    pub blackjack_table_repo: Arc<dyn crate::ports::outbound::casino::blackjack_table_repository::BlackjackTableRepository>,
    /// Game Portal : use cases lifecycle serveurs Docker.
    pub game_servers_uc: Arc<dyn ManageGameServersUseCase>,
    pub game_templates_uc: Arc<dyn ManageGameTemplatesUseCase>,
    /// Game Portal : adapters exposes pour les endpoints internes /jobs/*
    /// appeles par game-portal-worker (health/idle/reconcile).
    pub game_server_repo: Arc<dyn crate::ports::outbound::game::game_server_repository::GameServerRepository>,
    pub game_template_repo: Arc<dyn crate::ports::outbound::game::game_template_repository::GameTemplateRepository>,
    pub game_audit_repo: Arc<dyn crate::ports::outbound::game::game_audit_repository::GameAuditRepository>,
    pub game_session_repo: Arc<dyn crate::ports::outbound::game::player_session_repository::PlayerSessionRepository>,
    pub game_container_runtime: Arc<dyn crate::ports::outbound::game::container_runtime::ContainerRuntime>,
    pub game_rcon_client: Arc<dyn crate::ports::outbound::game::rcon_client::RconClient>,
    pub game_port_allocator: Arc<dyn crate::ports::outbound::game::port_allocator::PortAllocator>,
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
}

impl AppState {
    /// Lit le delai de rappel avant expiration depuis la config guild
    /// (cle `reminder_advance_secs` du bot `moderation-bot`). Default 3600s = 1h.
    pub async fn bot_config_reminder_advance_secs(&self, guild_id: &str) -> u64 {
        match self.bot_config_repo.get_config(guild_id, "moderation-bot").await {
            Ok(entries) => entries
                .iter()
                .find(|e| e.config_key == "reminder_advance_secs")
                .and_then(|e| e.config_value.parse().ok())
                .unwrap_or(3600),
            Err(_) => 3600,
        }
    }
}

use std::sync::Arc;

use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::job_client::JobClient;
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::domain::services::{DiscordApiService, InferenceService};
use crate::ports::inbound::{
    AnalyzeImageUseCase, AnalyzeMessageUseCase, ManageInfractionsUseCase, ManageModerationUseCase,
    ManageNotesUseCase, ManageRemindersUseCase, ManageRulesUseCase, ManageSecurityUseCase, ManageStatsUseCase, ManageStrikesUseCase, ManageTicketsUseCase,
    ManageAuditLogsUseCase, ManageConductUseCase, ManageLevelsUseCase, ManageMembersUseCase, ManageRolePanelsUseCase, ManageVoiceChannelsUseCase, ManageWatchedUsersUseCase,
};
use crate::ports::inbound::manage_coude_players::ManageCoudePlayersUseCase;
use crate::ports::inbound::manage_coude_combats::ManageCoudeCombatsUseCase;
use crate::ports::inbound::manage_coude_bets::ManageCoudeBetsUseCase;
use crate::ports::inbound::manage_coude_economy::ManageCoudeEconomyUseCase;
use crate::ports::inbound::manage_coude_inventory::ManageCoudeInventoryUseCase;
use crate::ports::inbound::manage_coude_social::ManageCoudeSocialUseCase;
use crate::application::BlackjackService;
use crate::ports::outbound::{AnalyticsRepository, BotConfigRepository, DailyActivityRepository, DiscordRoleRepository, GuildRepository, IaConfigRepository, LogRepository, WalletRepository};

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
    pub ia_config_repo: Arc<dyn IaConfigRepository>,
    pub discord_role_repo: Arc<dyn DiscordRoleRepository>,
    pub wallet_repo: Arc<dyn WalletRepository>,
    pub blackjack_svc: Arc<BlackjackService>,
    pub coude_players_uc: Arc<dyn ManageCoudePlayersUseCase>,
    pub coude_combats_uc: Arc<dyn ManageCoudeCombatsUseCase>,
    pub coude_bets_uc: Arc<dyn ManageCoudeBetsUseCase>,
    pub coude_economy_uc: Arc<dyn ManageCoudeEconomyUseCase>,
    pub coude_inventory_uc: Arc<dyn ManageCoudeInventoryUseCase>,
    pub coude_social_uc: Arc<dyn ManageCoudeSocialUseCase>,
    pub resolve_betting_batch_uc: Arc<dyn crate::ports::inbound::ResolveBettingBatchUseCase>,
    pub expire_combats_batch_uc: Arc<dyn crate::ports::inbound::ExpireCombatsBatchUseCase>,
    pub resolve_combat_now_uc: Arc<dyn crate::ports::inbound::ResolveCombatNowUseCase>,
    pub coude_catalog_uc: Arc<dyn crate::ports::inbound::ManageCoudeCatalogUseCase>,
    pub coude_cashbox_uc: Arc<dyn crate::ports::inbound::ManageCoudeCashboxUseCase>,
    pub coude_steal_protections_uc:
        Arc<dyn crate::ports::inbound::ManageCoudeStealProtectionsUseCase>,
    pub coude_steal_boosts_uc: Arc<dyn crate::ports::inbound::ManageCoudeStealBoostsUseCase>,
    pub coude_taunts_uc: Arc<dyn crate::ports::inbound::ManageCoudeTauntsUseCase>,
    pub coude_heist_uc: Arc<dyn crate::ports::inbound::ManageCoudeHeistUseCase>,
    pub broadcaster: Arc<EventBroadcaster>,
    #[allow(dead_code)]
    pub job_client: JobClient,
    pub discord_api: Arc<DiscordApiService>,
    pub inference: Arc<InferenceService>,
    pub api_key: String,
    #[allow(dead_code)]
    pub discord_bot_token: String,
    pub user_activity_repo: Arc<dyn crate::ports::outbound::UserActivityRepository>,
    pub welcome_config_repo: Arc<dyn crate::ports::outbound::WelcomeConfigRepository>,
    pub export_uc: Arc<dyn crate::application::ExecuteExportUseCase>,
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

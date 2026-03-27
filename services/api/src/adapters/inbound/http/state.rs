use std::sync::Arc;

use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::adapters::outbound::job_client::JobClient;
use crate::ports::inbound::{
    AnalyzeImageUseCase, AnalyzeMessageUseCase, ManageInfractionsUseCase, ManageModerationUseCase,
    ManageRulesUseCase, ManageSecurityUseCase, ManageStatsUseCase, ManageTicketsUseCase,
    ManageAuditLogsUseCase, ManageConductUseCase, ManageLevelsUseCase, ManageRolePanelsUseCase, ManageVoiceChannelsUseCase, ManageWatchedUsersUseCase,
};
use crate::ports::outbound::{AnalyticsRepository, BotConfigRepository, DailyActivityRepository, GuildRepository, IaConfigRepository, LogRepository};

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
    pub analytics_repo: Arc<dyn AnalyticsRepository>,
    pub daily_activity_repo: Arc<dyn DailyActivityRepository>,
    pub log_repo: Arc<dyn LogRepository>,
    pub guild_repo: Arc<dyn GuildRepository>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    pub ia_config_repo: Arc<dyn IaConfigRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
    pub job_client: JobClient,
    pub api_key: String,
    pub pg_pool: sqlx::PgPool,
    pub redis_client: redis::Client,
}

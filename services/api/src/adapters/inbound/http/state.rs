use std::sync::Arc;

use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::ports::inbound::{
    AnalyzeMessageUseCase, ManageInfractionsUseCase, ManageModerationUseCase,
    ManageRulesUseCase, ManageSecurityUseCase, ManageStatsUseCase, ManageTicketsUseCase,
    ManageConductUseCase, ManageVoiceChannelsUseCase,
};
use crate::ports::outbound::{BotConfigRepository, GuildRepository, LogRepository};

#[derive(Clone)]
pub struct AppState {
    pub analyze_uc: Arc<dyn AnalyzeMessageUseCase>,
    pub rules_uc: Arc<dyn ManageRulesUseCase>,
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    pub tickets_uc: Arc<dyn ManageTicketsUseCase>,
    pub security_uc: Arc<dyn ManageSecurityUseCase>,
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    pub stats_uc: Arc<dyn ManageStatsUseCase>,
    pub voice_channels_uc: Arc<dyn ManageVoiceChannelsUseCase>,
    pub conduct_uc: Arc<dyn ManageConductUseCase>,
    pub log_repo: Arc<dyn LogRepository>,
    pub guild_repo: Arc<dyn GuildRepository>,
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
    pub api_key: String,
    pub pg_pool: sqlx::PgPool,
    pub redis_client: redis::Client,
}

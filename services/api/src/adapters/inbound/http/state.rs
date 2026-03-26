use std::sync::Arc;

use crate::adapters::inbound::ws::broadcaster::EventBroadcaster;
use crate::ports::inbound::{
    AnalyzeMessageUseCase, ManageInfractionsUseCase, ManageModerationUseCase,
    ManageRulesUseCase, ManageSecurityUseCase, ManageTicketsUseCase,
};

#[derive(Clone)]
pub struct AppState {
    pub analyze_uc: Arc<dyn AnalyzeMessageUseCase>,
    pub rules_uc: Arc<dyn ManageRulesUseCase>,
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    pub tickets_uc: Arc<dyn ManageTicketsUseCase>,
    pub security_uc: Arc<dyn ManageSecurityUseCase>,
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    pub broadcaster: Arc<EventBroadcaster>,
    pub api_key: String,
}

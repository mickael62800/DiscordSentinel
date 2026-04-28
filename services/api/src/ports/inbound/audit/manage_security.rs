use async_trait::async_trait;

use crate::domain::entities::audit::security_event::SecurityEvent;
use crate::domain::errors::DomainError;
use crate::domain::services::audit::security_analyzer::JoinInfo;

pub struct ReportSecurityEventCommand {
    pub guild_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub user_ids: Vec<String>,
}

pub struct AnalyzeNewMemberCommand {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub has_avatar: bool,
    pub account_created_timestamp: i64,
    pub is_bot: bool,
    pub recent_joins: Vec<JoinInfo>,
}

/// Decision de securite retournee par l'API apres analyse.
#[derive(Debug, Clone, Default)]
pub struct SecurityDecision {
    pub is_raid: bool,
    pub raid_score: u32,
    pub is_suspicious_account: bool,
    pub is_alt_account: bool,
    pub alt_similar_to: String,
    pub quarantine: bool,
    pub send_captcha: bool,
    pub activate_lockdown: bool,
    pub slowmode_secs: u32,
    pub event_type: String,
    pub event_description: String,
}

#[async_trait]
pub trait ManageSecurityUseCase: Send + Sync {
    async fn report_event(
        &self,
        command: ReportSecurityEventCommand,
    ) -> Result<SecurityEvent, DomainError>;
    async fn list_events(&self, guild_id: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError>;

    /// Analyse un nouveau membre : raid, compte suspect, alt detection.
    /// L'API decide de tout et retourne les actions a executer par le bot.
    async fn analyze_new_member(
        &self,
        command: AnalyzeNewMemberCommand,
    ) -> Result<SecurityDecision, DomainError>;
}

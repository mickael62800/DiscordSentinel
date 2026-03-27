use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{Infraction, MessageAnalysis};
use crate::domain::errors::DomainError;
use crate::domain::services::ScoringService;
use crate::ports::inbound::{AnalyzeMessageCommand, AnalyzeMessageUseCase, DeductPointsCommand, ManageConductUseCase};
use crate::ports::outbound::{CachePort, InfractionRepository, RuleRepository};

pub struct AnalyzeMessageService {
    rule_repo: Arc<dyn RuleRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
    conduct_uc: Arc<dyn ManageConductUseCase>,
}

impl AnalyzeMessageService {
    pub fn new(
        rule_repo: Arc<dyn RuleRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
        conduct_uc: Arc<dyn ManageConductUseCase>,
    ) -> Self {
        Self {
            rule_repo,
            infraction_repo,
            cache,
            conduct_uc,
        }
    }
}

#[async_trait]
impl AnalyzeMessageUseCase for AnalyzeMessageService {
    async fn analyze(&self, cmd: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
        // 1. Charger les règles (cache → DB)
        let rules = match self.cache.get_rules(&cmd.guild_id).await? {
            Some(cached) => cached,
            None => {
                let from_db = self.rule_repo.find_by_guild(&cmd.guild_id).await?;
                self.cache.set_rules(&cmd.guild_id, &from_db).await.ok();
                from_db
            }
        };

        // 2. Scoring (logique métier pure)
        let result = ScoringService::score(&cmd.flags, &rules);

        // 3. Persister l'infraction
        let infraction = Infraction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            channel_id: cmd.channel_id,
            user_id: cmd.user_id,
            username: cmd.username,
            message_id: cmd.message_id,
            content: cmd.content,
            flags: cmd.flags,
            score: result.score,
            action: result.action.clone(),
            reason: result.reason.clone(),
            duration: result.duration,
            created_at: chrono::Utc::now(),
        };

        self.infraction_repo.save(&infraction).await?;

        // 3b. Deduire les points de conduite
        if result.action.as_str() != "none" {
            let _ = self.conduct_uc.deduct_points(DeductPointsCommand {
                guild_id: infraction.guild_id.clone(),
                user_id: infraction.user_id.clone(),
                username: infraction.username.clone(),
                action: result.action.as_str().to_string(),
            }).await;
        }

        // 4. Retourner l'analyse
        Ok(MessageAnalysis {
            action: result.action,
            reason: result.reason,
            score: result.score,
            duration: result.duration,
        })
    }
}

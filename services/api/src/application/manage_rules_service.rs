use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::Rule;
use crate::domain::errors::DomainError;
use crate::ports::inbound::{CreateRuleCommand, ManageRulesUseCase};
use crate::ports::outbound::{CachePort, RuleRepository};

pub struct ManageRulesService {
    rule_repo: Arc<dyn RuleRepository>,
    cache: Arc<dyn CachePort>,
}

impl ManageRulesService {
    pub fn new(rule_repo: Arc<dyn RuleRepository>, cache: Arc<dyn CachePort>) -> Self {
        Self { rule_repo, cache }
    }
}

#[async_trait]
impl ManageRulesUseCase for ManageRulesService {
    async fn get_rules(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError> {
        self.rule_repo.find_by_guild(guild_id).await
    }

    async fn create_or_update_rule(&self, cmd: CreateRuleCommand) -> Result<Rule, DomainError> {
        if cmd.weight < 0.0 {
            return Err(DomainError::InvalidRule("Le poids ne peut pas être négatif".into()));
        }
        if cmd.threshold_warn >= cmd.threshold_delete
            || cmd.threshold_delete >= cmd.threshold_mute
            || cmd.threshold_mute >= cmd.threshold_ban
        {
            return Err(DomainError::InvalidRule(
                "Les seuils doivent être croissants : warn < delete < mute < ban".into(),
            ));
        }

        let now = chrono::Utc::now();
        let rule = Rule {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            flag_type: cmd.flag_type,
            weight: cmd.weight,
            threshold_warn: cmd.threshold_warn,
            threshold_delete: cmd.threshold_delete,
            threshold_mute: cmd.threshold_mute,
            threshold_ban: cmd.threshold_ban,
            enabled: cmd.enabled,
            created_at: now,
            updated_at: now,
        };

        let saved = self.rule_repo.save(&rule).await?;

        // Invalider le cache pour ce serveur
        self.cache.invalidate_rules(&cmd.guild_id).await.ok();

        Ok(saved)
    }

    async fn delete_rule(&self, guild_id: &str, rule_id: Uuid) -> Result<(), DomainError> {
        self.rule_repo.delete(rule_id).await?;
        self.cache.invalidate_rules(guild_id).await.ok();
        Ok(())
    }
}

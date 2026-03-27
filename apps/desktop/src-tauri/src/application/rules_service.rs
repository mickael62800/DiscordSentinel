use std::sync::Arc;

use crate::domain::entities::{ModerationRule, UpdateRuleParams};
use crate::domain::ports::RulesRepository;

pub struct RulesService {
    repo: Arc<dyn RulesRepository>,
}

impl RulesService {
    pub fn new(repo: Arc<dyn RulesRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_rules(&self, guild_id: Option<String>) -> Result<Vec<ModerationRule>, String> {
        self.repo.get_rules(guild_id).await
    }

    pub async fn toggle_rule(&self, id: String, enabled: bool) -> Result<bool, String> {
        self.repo.toggle_rule(id, enabled).await
    }

    pub async fn update_rule(&self, params: UpdateRuleParams) -> Result<(), String> {
        self.repo.update_rule(params).await
    }
}

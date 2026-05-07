use async_trait::async_trait;
use uuid::Uuid;

use sentinel_core::domain::entities::system::rule::Rule;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::domain::enums::moderation::flag_type::FlagType;
use sentinel_core::domain::entities::system::discord_ids::GuildId;

pub struct CreateRuleCommand {
    pub guild_id: GuildId,
    pub flag_type: FlagType,
    pub weight: f64,
    pub threshold_warn: f64,
    pub threshold_delete: f64,
    pub threshold_mute: f64,
    pub threshold_ban: f64,
    pub enabled: bool,
}

#[async_trait]
pub trait ManageRulesUseCase: Send + Sync {
    async fn get_rules(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError>;
    async fn get_all_rules(&self) -> Result<Vec<Rule>, DomainError>;
    async fn toggle_rule(&self, rule_id: Uuid, enabled: bool) -> Result<bool, DomainError>;
    async fn create_or_update_rule(&self, command: CreateRuleCommand) -> Result<Rule, DomainError>;
    async fn delete_rule(&self, guild_id: &str, rule_id: Uuid) -> Result<(), DomainError>;
}

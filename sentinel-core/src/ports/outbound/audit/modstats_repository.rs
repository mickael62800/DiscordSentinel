use async_trait::async_trait;

use crate::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct ModeratorStat {
    pub moderator_id: String,
    pub moderator_name: String,
    pub action_count: i64,
}

#[async_trait]
pub trait ModstatsRepository: Send + Sync {
    async fn top_moderators(&self, guild_id: &str, days: i32, limit: i64) -> Result<Vec<ModeratorStat>, DomainError>;
}

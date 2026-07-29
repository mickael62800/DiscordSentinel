use crate::{domain::errors::DomainError, ports::outbound::coude_inventory_repository::InventoryItem};
use async_trait::async_trait;
#[async_trait]
pub trait CoudeInventoryUseCase: Send + Sync {
    async fn inventory(&self, guild_id: &str, user_id: &str) -> Result<Vec<InventoryItem>, DomainError>;
    async fn buy(&self, guild_id: &str, user_id: &str, item_key: &str) -> Result<i64, DomainError>;
}

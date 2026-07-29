use std::sync::Arc;
use async_trait::async_trait;
use crate::{domain::{entities::coude_shop::item, errors::DomainError}, ports::{inbound::coude_inventory::CoudeInventoryUseCase, outbound::coude_inventory_repository::{CoudeInventoryRepository, InventoryItem}}};
pub struct CoudeInventoryService { repo: Arc<dyn CoudeInventoryRepository> }
impl CoudeInventoryService { pub fn new(repo: Arc<dyn CoudeInventoryRepository>) -> Self { Self { repo } } }
#[async_trait]
impl CoudeInventoryUseCase for CoudeInventoryService {
    async fn inventory(&self, guild_id: &str, user_id: &str) -> Result<Vec<InventoryItem>, DomainError> { self.repo.list(guild_id, user_id).await }
    async fn buy(&self, guild_id: &str, user_id: &str, item_key: &str) -> Result<i64, DomainError> {
        let item = item(item_key).ok_or_else(|| DomainError::Validation("objet inconnu".into()))?;
        self.repo.buy(guild_id, user_id, item.key, item.price).await
    }
}

use std::sync::Arc;
use async_trait::async_trait;
use crate::{domain::{entities::coussin_shop::item, errors::DomainError}, ports::{inbound::coussin_inventory::CoussinInventoryUseCase, outbound::coussin_inventory_repository::{CoussinInventoryRepository, InventoryItem}}};
pub struct CoussinInventoryService { repo: Arc<dyn CoussinInventoryRepository> }
impl CoussinInventoryService { pub fn new(repo: Arc<dyn CoussinInventoryRepository>) -> Self { Self { repo } } }
#[async_trait]
impl CoussinInventoryUseCase for CoussinInventoryService {
    async fn inventory(&self, guild_id: &str, user_id: &str) -> Result<Vec<InventoryItem>, DomainError> { self.repo.list(guild_id, user_id).await }
    async fn buy(&self, guild_id: &str, user_id: &str, item_key: &str) -> Result<i64, DomainError> {
        let item = item(item_key).ok_or_else(|| DomainError::Validation("objet inconnu".into()))?;
        self.repo.buy(guild_id, user_id, item.key, item.price).await
    }
}

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{
    CoudeInsurance, CoudeInventoryItem, CoudePrime, NewCoudePrime,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_inventory::ManageCoudeInventoryUseCase;
use crate::ports::outbound::CoudeInventoryRepository;

pub struct ManageCoudeInventoryService {
    repo: Arc<dyn CoudeInventoryRepository>,
}

impl ManageCoudeInventoryService {
    pub fn new(repo: Arc<dyn CoudeInventoryRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageCoudeInventoryUseCase for ManageCoudeInventoryService {
    async fn list_inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<CoudeInventoryItem>, DomainError> {
        self.repo.list_inventory(guild_id, user_id).await
    }

    async fn add_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<(), DomainError> {
        if item_key.trim().is_empty() {
            return Err(DomainError::ValidationError("item_key requis".into()));
        }
        self.repo.add_item(guild_id, user_id, item_key).await
    }

    async fn use_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, DomainError> {
        self.repo.use_item(guild_id, user_id, item_key).await
    }

    async fn has_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<bool, DomainError> {
        self.repo.has_item(guild_id, user_id, item_key).await
    }

    async fn create_prime(&self, new: NewCoudePrime) -> Result<CoudePrime, DomainError> {
        if new.amount <= 0 {
            return Err(DomainError::ValidationError(
                "Le montant d'une prime doit etre positif".into(),
            ));
        }
        if new.target_id == new.placed_by_id {
            return Err(DomainError::ValidationError(
                "Impossible de placer une prime sur soi-meme".into(),
            ));
        }
        self.repo.create_prime(new).await
    }

    async fn list_active_primes(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Vec<CoudePrime>, DomainError> {
        self.repo.list_active_primes(guild_id, target_id).await
    }

    async fn claim_primes(
        &self,
        guild_id: &str,
        target_id: &str,
        claimer_id: &str,
        claimer_name: &str,
    ) -> Result<i64, DomainError> {
        self.repo
            .claim_primes(guild_id, target_id, claimer_id, claimer_name)
            .await
    }

    async fn buy_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
        is_scam: bool,
    ) -> Result<(), DomainError> {
        self.repo.buy_insurance(guild_id, user_id, is_scam).await
    }

    async fn get_active_insurance(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudeInsurance>, DomainError> {
        self.repo.get_active_insurance(guild_id, user_id).await
    }

    async fn expire_insurance(&self, insurance_id: Uuid) -> Result<(), DomainError> {
        let expired = self.repo.expire_insurance(insurance_id).await?;
        if !expired {
            return Err(DomainError::NotFound("Assurance introuvable".into()));
        }
        Ok(())
    }
}

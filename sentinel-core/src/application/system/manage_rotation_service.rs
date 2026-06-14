use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::system::admin_rotation::{RotationState, ServedEntry};
use crate::domain::errors::DomainError;
use crate::ports::inbound::system::manage_rotation::ManageRotationUseCase;
use crate::ports::outbound::system::admin_rotation_repository::AdminRotationRepository;

pub struct ManageRotationService {
    repo: Arc<dyn AdminRotationRepository>,
}

impl ManageRotationService {
    pub fn new(repo: Arc<dyn AdminRotationRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageRotationUseCase for ManageRotationService {
    async fn get_state(&self, guild_id: &str) -> Result<RotationState, DomainError> {
        Ok(self
            .repo
            .get(guild_id)
            .await?
            .unwrap_or_else(|| RotationState::idle(guild_id)))
    }

    async fn save_state(&self, state: RotationState) -> Result<(), DomainError> {
        if state.guild_id.trim().is_empty() {
            return Err(DomainError::ValidationError("guild_id requis".into()));
        }
        self.repo.upsert(&state).await
    }

    async fn record_served(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.repo.record_served(guild_id, user_id).await
    }

    async fn served_entries(&self, guild_id: &str) -> Result<Vec<ServedEntry>, DomainError> {
        self.repo.served_entries(guild_id).await
    }
}

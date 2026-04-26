//! Service filet de securite (cf. COUPE_AMELIORATIONS 4.4).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::{
    safety_net_should_trigger, ActiveSafetyNet, SAFETY_NET_DURATION_HOURS,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_safety_net::ManageCoudeSafetyNetUseCase;
use crate::ports::outbound::CoudeSafetyNetRepository;

pub struct ManageCoudeSafetyNetService {
    repo: Arc<dyn CoudeSafetyNetRepository>,
}

impl ManageCoudeSafetyNetService {
    pub fn new(repo: Arc<dyn CoudeSafetyNetRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageCoudeSafetyNetUseCase for ManageCoudeSafetyNetService {
    async fn try_activate(
        &self,
        guild_id: &str,
        user_id: &str,
        current_balance: i64,
    ) -> Result<Option<ActiveSafetyNet>, DomainError> {
        if !safety_net_should_trigger(current_balance) {
            return Ok(None);
        }
        // Skip si filet deja actif (pas de cumul).
        if self.repo.get_active(guild_id, user_id).await?.is_some() {
            return Ok(None);
        }
        let _id = self
            .repo
            .activate(guild_id, user_id, SAFETY_NET_DURATION_HOURS)
            .await?;
        // Re-read pour avoir les timestamps definitifs.
        self.repo.get_active(guild_id, user_id).await
    }

    async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<ActiveSafetyNet>, DomainError> {
        self.repo.get_active(guild_id, user_id).await
    }

    async fn list_active(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ActiveSafetyNet>, DomainError> {
        self.repo.list_active(guild_id).await
    }
}

#[cfg(test)]
#[path = "tests/manage_coude_safety_net.rs"]
mod tests;

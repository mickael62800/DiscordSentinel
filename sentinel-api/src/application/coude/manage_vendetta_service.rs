//! Service vendetta (cf. COUPE_AMELIORATIONS 5.3).

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::vendetta::ActiveVendetta;
use sentinel_core::domain::entities::coude::vendetta::VENDETTA_WINDOW_HOURS;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_vendetta::ManageCoudeVendettaUseCase;
use crate::ports::outbound::coude::vendetta_repository::VendettaRepository;

pub struct ManageCoudeVendettaService {
    repo: Arc<dyn VendettaRepository>,
}

impl ManageCoudeVendettaService {
    pub fn new(repo: Arc<dyn VendettaRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageCoudeVendettaUseCase for ManageCoudeVendettaService {
    async fn declare(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
    ) -> Result<Uuid, DomainError> {
        if challenger_id == target_id {
            return Err(DomainError::ValidationError(
                "Tu ne peux pas declarer une vendetta contre toi-meme.".into(),
            ));
        }
        // Garde-fou avant insert : evite l erreur unique violation si une
        // vendetta active existe deja sur ce couple.
        if self
            .repo
            .get_active(guild_id, challenger_id, target_id)
            .await?
            .is_some()
        {
            return Err(DomainError::Conflict(
                "Une vendetta est deja active contre cette cible.".into(),
            ));
        }
        self.repo
            .declare(guild_id, challenger_id, target_id, VENDETTA_WINDOW_HOURS)
            .await
    }

    async fn get_active(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveVendetta>, DomainError> {
        self.repo
            .get_active(guild_id, challenger_id, target_id)
            .await
    }

    async fn resolve(&self, id: Uuid, won: bool) -> Result<(), DomainError> {
        self.repo.resolve(id, won).await
    }

    async fn list_by_challenger(
        &self,
        guild_id: &str,
        challenger_id: &str,
    ) -> Result<Vec<ActiveVendetta>, DomainError> {
        self.repo.list_by_challenger(guild_id, challenger_id).await
    }
}

#[cfg(test)]
#[path = "tests/manage_vendetta.rs"]
mod tests;

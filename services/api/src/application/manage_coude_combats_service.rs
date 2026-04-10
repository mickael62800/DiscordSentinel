use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{CombatResolution, CoudeCombat, NewCoudeCombat};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_coude_combats::ManageCoudeCombatsUseCase;
use crate::ports::outbound::CoudeCombatRepository;

pub struct ManageCoudeCombatsService {
    repo: Arc<dyn CoudeCombatRepository>,
}

impl ManageCoudeCombatsService {
    pub fn new(repo: Arc<dyn CoudeCombatRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageCoudeCombatsUseCase for ManageCoudeCombatsService {
    async fn list(
        &self,
        guild_id: &str,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CoudeCombat>, DomainError> {
        let limit = limit.clamp(1, 200);
        // "all" est traité côté repo via Option::None.
        let status_filter = status.filter(|s| *s != "all");
        self.repo.list(guild_id, status_filter, limit).await
    }

    async fn get(&self, id: Uuid) -> Result<CoudeCombat, DomainError> {
        self.repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Combat introuvable".into()))
    }

    async fn get_pending_for_attacker(
        &self,
        guild_id: &str,
        attacker_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError> {
        self.repo.get_pending_for_attacker(guild_id, attacker_id).await
    }

    async fn get_pending_for_defender(
        &self,
        guild_id: &str,
        defender_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError> {
        self.repo.get_pending_for_defender(guild_id, defender_id).await
    }

    async fn list_expired_pending(&self) -> Result<Vec<CoudeCombat>, DomainError> {
        self.repo.list_expired_pending().await
    }

    async fn get_betting_for_participant(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudeCombat>, DomainError> {
        self.repo
            .get_betting_for_participant(guild_id, user_id)
            .await
    }

    async fn create(&self, new: NewCoudeCombat) -> Result<CoudeCombat, DomainError> {
        if new.mise < 0 {
            return Err(DomainError::ValidationError(
                "La mise ne peut pas etre negative".into(),
            ));
        }
        if new.attacker_id == new.defender_id {
            return Err(DomainError::ValidationError(
                "Un joueur ne peut pas se defier lui-meme".into(),
            ));
        }
        self.repo.create(new).await
    }

    async fn cancel(&self, id: Uuid) -> Result<(), DomainError> {
        let cancelled = self.repo.cancel_pending(id).await?;
        if !cancelled {
            return Err(DomainError::NotFound(
                "Combat introuvable ou deja resolu".into(),
            ));
        }

        // Effet de bord : marquer les paris non résolus comme perdus.
        // Si ça échoue, on log mais on ne fait pas échouer la commande
        // (le combat est déjà annulé, mieux vaut un état partiellement
        // cohérent qu'une 500 qui laisse l'utilisateur dans le doute).
        if let Err(e) = self.repo.mark_unresolved_bets_lost(id).await {
            tracing::warn!(
                error = %e,
                combat_id = %id,
                "Echec remboursement paris apres annulation combat"
            );
        }

        Ok(())
    }

    async fn resolve(
        &self,
        id: Uuid,
        resolution: CombatResolution,
    ) -> Result<(), DomainError> {
        let resolved = self.repo.resolve(id, resolution).await?;
        if !resolved {
            return Err(DomainError::Conflict(
                "Combat deja resolu ou introuvable".into(),
            ));
        }
        Ok(())
    }

    async fn set_betting(&self, id: Uuid, message_id: &str) -> Result<bool, DomainError> {
        if message_id.is_empty() {
            return Err(DomainError::ValidationError(
                "message_id requis".into(),
            ));
        }
        self.repo.set_betting(id, message_id).await
    }

    async fn expire(&self, id: Uuid) -> Result<(), DomainError> {
        let expired = self.repo.expire(id).await?;
        if !expired {
            return Err(DomainError::NotFound("Combat introuvable".into()));
        }
        Ok(())
    }

    async fn set_defender_special(
        &self,
        id: Uuid,
        item_key: &str,
    ) -> Result<(), DomainError> {
        if item_key.is_empty() {
            return Err(DomainError::ValidationError("item_key requis".into()));
        }
        let updated = self.repo.set_defender_special(id, item_key).await?;
        if !updated {
            return Err(DomainError::NotFound("Combat introuvable".into()));
        }
        Ok(())
    }
}

//! Use case tentatives de vol : genere l'id, calcule la fenetre de defense et
//! delegue les transitions de statut au `StealAttemptRepository`. Aucune
//! dependance infra (SQL) ici — domaine pur.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::entities::coude::steal::attempt::CreatedStealAttempt;
use crate::domain::entities::coude::steal::attempt::NewStealAttempt;
use crate::domain::errors::DomainError;
use crate::ports::inbound::coude::manage_steal_attempts::CreateStealAttempt;
use crate::ports::inbound::coude::manage_steal_attempts::ManageStealAttemptsUseCase;
use crate::ports::outbound::coude::steal_attempt_repository::StealAttemptRepository;

pub struct ManageStealAttemptsService {
    repo: Arc<dyn StealAttemptRepository>,
}

impl ManageStealAttemptsService {
    pub fn new(repo: Arc<dyn StealAttemptRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageStealAttemptsUseCase for ManageStealAttemptsService {
    async fn create_attempt(
        &self,
        cmd: CreateStealAttempt,
    ) -> Result<CreatedStealAttempt, DomainError> {
        let id = Uuid::new_v4();
        // Fenetre de defense : au moins 1s pour eviter une expiration immediate.
        let expires_at = Utc::now() + chrono::Duration::seconds(cmd.window_secs.max(1));

        self.repo
            .insert_pending(&NewStealAttempt {
                id,
                guild_id: cmd.guild_id,
                thief_id: cmd.thief_id,
                target_id: cmd.target_id,
                message_id: cmd.message_id,
                channel_id: cmd.channel_id,
                expires_at,
            })
            .await?;

        Ok(CreatedStealAttempt { id, expires_at })
    }

    async fn mark_defended(&self, id: Uuid) -> Result<(), DomainError> {
        // La transition est idempotente cote appelant : qu'elle ait eu lieu ou
        // non (deja defended/expired/resolved), on renvoie Ok.
        self.repo.mark_defended(id).await?;
        Ok(())
    }

    async fn claim_resolved(&self, id: Uuid) -> Result<bool, DomainError> {
        self.repo.claim_resolved(id).await
    }
}

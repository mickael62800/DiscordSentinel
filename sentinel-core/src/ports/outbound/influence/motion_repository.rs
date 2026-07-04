//! Ports outbound : motions et bulletins de vote.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::influence::motion::{Motion, MotionStatus};
use crate::domain::entities::influence::vote::{Tally, VoteChoice};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait MotionRepository: Send + Sync {
    /// Cree une motion (statut « ouverte »).
    async fn create(
        &self,
        guild_id: &str,
        org_id: Uuid,
        title: &str,
        created_by: Uuid,
    ) -> Result<Motion, DomainError>;

    async fn get(&self, id: Uuid) -> Result<Option<Motion>, DomainError>;

    /// Change le statut d'une motion (cloture).
    async fn set_status(&self, id: Uuid, status: MotionStatus) -> Result<(), DomainError>;
}

#[async_trait]
pub trait VoteRepository: Send + Sync {
    /// Enregistre (ou remplace) le bulletin d'un votant sur une motion.
    async fn upsert(
        &self,
        subject_id: Uuid,
        voter_id: Uuid,
        choice: VoteChoice,
    ) -> Result<(), DomainError>;

    /// Decompte des bulletins d'une motion.
    async fn tally(&self, subject_id: Uuid) -> Result<Tally, DomainError>;
}

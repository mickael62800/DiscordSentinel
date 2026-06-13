use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::moderation::review::automod::AutomodReview;
use crate::domain::entities::moderation::review::automod::NewAutomodReview;
use crate::domain::entities::moderation::review::automod::ReviewVote;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait AutomodReviewRepository: Send + Sync {
    async fn create(&self, review: NewAutomodReview) -> Result<AutomodReview, DomainError>;
    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError>;
    async fn list_pending(&self, guild_id: &str, limit: i64) -> Result<Vec<AutomodReview>, DomainError>;
    async fn list_recent(&self, guild_id: &str, limit: i64) -> Result<Vec<AutomodReview>, DomainError>;
    /// Resolve une review (statut pending OU decided -> applied|ignored).
    /// Retourne la review mise a jour ou `Conflict` si deja resolue.
    async fn resolve(
        &self,
        id: Uuid,
        applied_action: &str,
        resolved_by_id: &str,
        resolved_by_name: &str,
        resolved_source: &str,
    ) -> Result<AutomodReview, DomainError>;

    // ── Vote ──
    /// Enregistre/met a jour le vote d'un moderateur (un seul par review et
    /// par votant). `Conflict` si la review n'est plus en statut 'voting'.
    async fn upsert_vote(
        &self,
        review_id: Uuid,
        voter_id: &str,
        voter_name: &str,
        vote_action: &str,
    ) -> Result<(), DomainError>;

    /// Liste les votes d'une review.
    async fn list_votes(&self, review_id: Uuid) -> Result<Vec<ReviewVote>, DomainError>;

    /// Passe une review de 'voting' a 'decided' avec le verdict calcule.
    /// `Conflict` si la review n'est plus en 'voting'.
    async fn decide(
        &self,
        id: Uuid,
        decided_action: &str,
        quorum_met: bool,
    ) -> Result<AutomodReview, DomainError>;

    /// Reviews en statut 'voting' dont l'echeance est depassee (job worker).
    async fn list_expired_voting(&self, limit: i64) -> Result<Vec<AutomodReview>, DomainError>;
}

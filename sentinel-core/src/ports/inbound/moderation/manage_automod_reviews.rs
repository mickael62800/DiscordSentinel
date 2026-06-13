//! Use case Automod review (cards moderation).
//!
//! Le HTTP handler appelle ce port — jamais directement le repo. Permet
//! d'isoler la regle metier (idempotence, vote, depouillement) du transport.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::moderation::review::automod::AutomodReview;
use crate::domain::entities::moderation::review::automod::NewAutomodReview;
use crate::domain::entities::moderation::review::automod::ReviewVote;
use crate::domain::entities::moderation::review::automod::TallyResult;
use crate::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct ResolveAutomodReviewCommand {
    pub review_id: Uuid,
    /// Action choisie : "warn", "delete", "mute", "ban", "ignore".
    pub applied_action: String,
    pub resolved_by_id: String,
    pub resolved_by_name: String,
    /// "discord" ou "web".
    pub resolved_source: String,
}

/// Vote d'un moderateur sur une review en cours.
#[derive(Debug, Clone)]
pub struct CastVoteCommand {
    pub review_id: Uuid,
    pub voter_id: String,
    pub voter_name: String,
    /// "warn" | "delete" | "mute" | "ban" | "ignore".
    pub vote_action: String,
}

#[async_trait]
pub trait ManageAutomodReviewsUseCase: Send + Sync {
    async fn create(&self, review: NewAutomodReview) -> Result<AutomodReview, DomainError>;
    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError>;
    async fn list_pending(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError>;
    async fn list_recent(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError>;
    async fn resolve(
        &self,
        cmd: ResolveAutomodReviewCommand,
    ) -> Result<AutomodReview, DomainError>;

    // ── Vote ──
    /// Enregistre/met a jour un vote, retourne la liste des votes a jour.
    async fn cast_vote(&self, cmd: CastVoteCommand) -> Result<Vec<ReviewVote>, DomainError>;

    /// Liste les votes d'une review.
    async fn list_votes(&self, review_id: Uuid) -> Result<Vec<ReviewVote>, DomainError>;

    /// Cloture le vote : depouille (quorum + tie-break) et passe en
    /// 'decided'. Retourne la review et le resultat du depouillement.
    async fn decide(
        &self,
        review_id: Uuid,
        quorum: usize,
        tie_action: &str,
    ) -> Result<(AutomodReview, TallyResult), DomainError>;

    /// Reviews en vote dont l'echeance est depassee (job worker).
    async fn list_expired_voting(&self, limit: i64) -> Result<Vec<AutomodReview>, DomainError>;
}

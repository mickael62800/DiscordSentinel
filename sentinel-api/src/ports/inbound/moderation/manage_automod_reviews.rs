//! Use case Automod review (cards moderation).
//!
//! Le HTTP handler appelle ce port — jamais directement le repo. Permet
//! d'isoler la regle metier `pending -> applied|ignored` (idempotence) du
//! transport.

use async_trait::async_trait;
use uuid::Uuid;

use sentinel_core::domain::entities::moderation::review::automod::AutomodReview;
use sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview;
use sentinel_core::domain::errors::DomainError;

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
}

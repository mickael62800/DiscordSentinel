use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::moderation::review::automod::AutomodReview;
use crate::domain::entities::moderation::review::automod::NewAutomodReview;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait AutomodReviewRepository: Send + Sync {
    async fn create(&self, review: NewAutomodReview) -> Result<AutomodReview, DomainError>;
    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError>;
    async fn list_pending(&self, guild_id: &str, limit: i64) -> Result<Vec<AutomodReview>, DomainError>;
    async fn list_recent(&self, guild_id: &str, limit: i64) -> Result<Vec<AutomodReview>, DomainError>;
    /// Resolve une review pending. Retourne la review mise a jour ou
    /// `Conflict` si deja resolue (idempotence).
    async fn resolve(
        &self,
        id: Uuid,
        applied_action: &str,
        resolved_by_id: &str,
        resolved_by_name: &str,
        resolved_source: &str,
    ) -> Result<AutomodReview, DomainError>;
}

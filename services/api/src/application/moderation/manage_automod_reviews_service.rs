//! Service application Automod reviews.
//!
//! Regles :
//!   * `applied_action` doit etre une valeur reconnue ('warn'|'delete'|'mute'|'ban'|'ignore').
//!   * `resolved_source` doit etre 'web' ou 'discord'.
//!   * idempotence : la 2e resolve sur la meme review renvoie `Conflict`
//!     (le repo Postgres garantit ca via `UPDATE WHERE status='pending'`).

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::moderation::review::automod::AppliedAction;
use crate::domain::entities::moderation::review::automod::AutomodReview;
use crate::domain::entities::moderation::review::automod::NewAutomodReview;
use crate::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase;
use crate::ports::inbound::moderation::manage_automod_reviews::ResolveAutomodReviewCommand;
use crate::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository;

pub struct ManageAutomodReviewsService {
    repo: Arc<dyn AutomodReviewRepository>,
}

impl ManageAutomodReviewsService {
    pub fn new(repo: Arc<dyn AutomodReviewRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageAutomodReviewsUseCase for ManageAutomodReviewsService {
    async fn create(&self, review: NewAutomodReview) -> Result<AutomodReview, DomainError> {
        if review.guild_id.trim().is_empty() {
            return Err(DomainError::ValidationError("guild_id requis".into()));
        }
        self.repo.create(review).await
    }

    async fn get(&self, id: Uuid) -> Result<Option<AutomodReview>, DomainError> {
        self.repo.get(id).await
    }

    async fn list_pending(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        if guild_id.trim().is_empty() {
            return Err(DomainError::ValidationError("guild_id requis".into()));
        }
        self.repo.list_pending(guild_id, limit.clamp(1, 500)).await
    }

    async fn list_recent(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<AutomodReview>, DomainError> {
        if guild_id.trim().is_empty() {
            return Err(DomainError::ValidationError("guild_id requis".into()));
        }
        self.repo.list_recent(guild_id, limit.clamp(1, 500)).await
    }

    async fn resolve(
        &self,
        cmd: ResolveAutomodReviewCommand,
    ) -> Result<AutomodReview, DomainError> {
        if AppliedAction::from_str(&cmd.applied_action).is_none() {
            return Err(DomainError::ValidationError(format!(
                "applied_action invalide : {}. Valeurs : warn|delete|mute|ban|ignore",
                cmd.applied_action
            )));
        }
        if !matches!(cmd.resolved_source.as_str(), "web" | "discord") {
            return Err(DomainError::ValidationError(
                "resolved_source doit etre 'web' ou 'discord'".into(),
            ));
        }
        if cmd.resolved_by_id.trim().is_empty() {
            return Err(DomainError::ValidationError("resolved_by_id requis".into()));
        }
        self.repo
            .resolve(
                cmd.review_id,
                &cmd.applied_action,
                &cmd.resolved_by_id,
                &cmd.resolved_by_name,
                &cmd.resolved_source,
            )
            .await
    }
}

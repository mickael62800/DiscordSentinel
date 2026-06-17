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

use crate::domain::entities::moderation::review::automod::tally_votes;
use crate::domain::entities::moderation::review::automod::AppliedAction;
use crate::domain::entities::moderation::review::automod::AutomodReview;
use crate::domain::entities::moderation::review::automod::NewAutomodReview;
use crate::domain::entities::moderation::review::automod::ReviewVote;
use crate::domain::entities::moderation::review::automod::TallyResult;
use crate::domain::entities::moderation::review::automod::TieAction;
use crate::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_automod_reviews::CastVoteCommand;
use crate::ports::inbound::moderation::manage_automod_reviews::CloseIgnoredCommand;
use crate::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase;
use crate::ports::inbound::moderation::manage_automod_reviews::ReopenReviewCommand;
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

    async fn create_or_merge(
        &self,
        review: NewAutomodReview,
        aggregate: bool,
        window_minutes: i64,
    ) -> Result<(AutomodReview, bool), DomainError> {
        if review.guild_id.trim().is_empty() {
            return Err(DomainError::ValidationError("guild_id requis".into()));
        }
        self.repo.create_or_merge(review, aggregate, window_minutes).await
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
        // Regle d'acces (domaine) : finalisation Discord reservee aux admins.
        // La source "web" est autorisee en amont par le middleware guild_auth.
        if let Some(facts) = &cmd.requester {
            if !crate::domain::entities::moderation::review::automod::can_finalize_review(facts) {
                return Err(DomainError::Forbidden(
                    "Seul un administrateur peut finaliser.".into(),
                ));
            }
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

    async fn close_ignored(
        &self,
        cmd: CloseIgnoredCommand,
    ) -> Result<AutomodReview, DomainError> {
        if !matches!(cmd.source.as_str(), "web" | "discord") {
            return Err(DomainError::ValidationError(
                "source doit etre 'web' ou 'discord'".into(),
            ));
        }
        if cmd.actor_id.trim().is_empty() {
            return Err(DomainError::ValidationError("actor_id requis".into()));
        }
        // Regle d'acces (domaine) : tout moderateur peut clore (source discord).
        // La source "web" est autorisee en amont par le middleware guild_auth.
        if let Some(facts) = &cmd.requester {
            if !crate::domain::entities::moderation::review::automod::is_moderator(facts) {
                return Err(DomainError::Forbidden(
                    "Seul un moderateur peut clore ce dossier.".into(),
                ));
            }
        }
        self.repo
            .close_ignored(cmd.review_id, &cmd.actor_id, &cmd.actor_name, &cmd.source)
            .await
    }

    async fn reopen(&self, cmd: ReopenReviewCommand) -> Result<AutomodReview, DomainError> {
        if cmd.actor_id.trim().is_empty() {
            return Err(DomainError::ValidationError("actor_id requis".into()));
        }
        if let Some(facts) = &cmd.requester {
            if !crate::domain::entities::moderation::review::automod::is_moderator(facts) {
                return Err(DomainError::Forbidden(
                    "Seul un moderateur peut rouvrir ce dossier.".into(),
                ));
            }
        }
        let hours = cmd.deadline_hours.clamp(1, 720);
        self.repo.reopen(cmd.review_id, hours).await
    }

    async fn cast_vote(&self, cmd: CastVoteCommand) -> Result<Vec<ReviewVote>, DomainError> {
        // Regle d'acces (domaine) : seul un moderateur peut voter.
        if !crate::domain::entities::moderation::review::automod::is_moderator(&cmd.requester) {
            return Err(DomainError::Forbidden("Tu n'es pas autorise a voter.".into()));
        }
        if AppliedAction::from_str(&cmd.vote_action).is_none() {
            return Err(DomainError::ValidationError(format!(
                "vote_action invalide : {}. Valeurs : warn|delete|mute|ban|ignore",
                cmd.vote_action
            )));
        }
        if cmd.voter_id.trim().is_empty() {
            return Err(DomainError::ValidationError("voter_id requis".into()));
        }
        self.repo
            .upsert_vote(cmd.review_id, &cmd.voter_id, &cmd.voter_name, &cmd.vote_action)
            .await?;
        self.repo.list_votes(cmd.review_id).await
    }

    async fn list_votes(&self, review_id: Uuid) -> Result<Vec<ReviewVote>, DomainError> {
        self.repo.list_votes(review_id).await
    }

    async fn decide(
        &self,
        review_id: Uuid,
        quorum: usize,
        tie_action: &str,
    ) -> Result<(AutomodReview, TallyResult), DomainError> {
        let votes = self.repo.list_votes(review_id).await?;
        let actions: Vec<AppliedAction> = votes
            .iter()
            .filter_map(|v| AppliedAction::from_str(&v.vote_action))
            .collect();
        let tally = tally_votes(&actions, quorum, TieAction::from_str(tie_action));
        let review = self
            .repo
            .decide(review_id, tally.decided.as_str(), tally.quorum_met)
            .await?;
        Ok((review, tally))
    }

    async fn list_expired_voting(&self, limit: i64) -> Result<Vec<AutomodReview>, DomainError> {
        self.repo.list_expired_voting(limit.clamp(1, 500)).await
    }

    async fn get_discussion(
        &self,
        review_id: Uuid,
    ) -> Result<Option<crate::domain::entities::moderation::review::automod::DiscussionChannel>, DomainError> {
        self.repo.find_discussion(review_id).await
    }

    async fn open_discussion(
        &self,
        cmd: crate::ports::inbound::moderation::manage_automod_reviews::OpenDiscussionCommand,
    ) -> Result<(crate::domain::entities::moderation::review::automod::DiscussionChannel, bool), DomainError> {
        use crate::domain::entities::moderation::review::automod::{can_open_discussion, NewDiscussionChannel};

        // Regle d'acces (domaine) : le demandeur doit etre moderateur.
        if !can_open_discussion(&cmd.requester) {
            return Err(DomainError::Forbidden(
                "Tu n'es pas autorise a ouvrir une discussion.".into(),
            ));
        }
        if cmd.channel_id.trim().is_empty() {
            return Err(DomainError::ValidationError("channel_id requis".into()));
        }
        // Pas de discussion sur une affaire deja close (sanction appliquee ou ignoree).
        if let Some(review) = self.repo.get(cmd.review_id).await? {
            if matches!(review.status.as_str(), "applied" | "ignored") {
                return Err(DomainError::Conflict(
                    "Cette review est close : impossible d'ouvrir une discussion.".into(),
                ));
            }
        } else {
            return Err(DomainError::NotFound(format!(
                "review {} introuvable",
                cmd.review_id
            )));
        }
        self.repo
            .create_discussion(NewDiscussionChannel {
                review_id: cmd.review_id,
                guild_id: cmd.guild_id,
                channel_id: cmd.channel_id,
                opened_by_id: cmd.opened_by_id,
                opened_by_name: cmd.opened_by_name,
            })
            .await
    }
}

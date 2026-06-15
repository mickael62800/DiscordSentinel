//! Handler HTTP du module Automod (Phase 4).
//!
//! Pas de logique metier ici — on reutilise `ManageInfractionsUseCase`
//! (port inbound) avec un filtre `action="detection"`. La page
//! `/automod` cote web consomme ce endpoint pour la timeline des
//! detections automod.

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::adapters::inbound::http::dto::moderation::infractions::InfractionResponseDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::normalize_limit;
use crate::adapters::inbound::http::helpers::normalize_offset;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use sentinel_core::domain::entities::moderation::review::automod::AutomodReview;
use sentinel_core::domain::entities::moderation::review::automod::NewAutomodReview;
use sentinel_core::domain::entities::moderation::review::automod::SuggestedAction;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::ports::inbound::moderation::manage_automod_reviews::ResolveAutomodReviewCommand;
use sentinel_core::domain::entities::system::discord_ids::MessageId;
use sentinel_core::domain::entities::system::discord_ids::ChannelId;
use sentinel_core::domain::entities::system::discord_ids::UserId;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
#[derive(Debug, Deserialize)]
pub struct DetectionQuery {
    /// Defaut 50, max 200.
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Optionnel : filtre par utilisateur.
    pub user_id: Option<String>,
}

/// GET /api/automod/{guild_id}/detections
pub async fn list_detections(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<DetectionQuery>,
) -> Result<Json<Vec<InfractionResponseDto>>, ApiError> {
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;

    // Filtre `action = "detection"` : seules les detections automod, pas
    // les actions de moderation (warn/mute/ban...).
    let filters = InfractionFilters {
        user_id: params.user_id,
        action: Some("detection".to_string()),
        limit: normalize_limit(params.limit, 50, 200),
        offset: normalize_offset(params.offset),
    };

    let detections = state
        .infractions_uc
        .list_infractions(&guild_id, filters)
        .await?;
    Ok(map_to_dtos(detections))
}

/// DTO public d'une carte de review automod.
#[derive(Debug, Serialize)]
pub struct AutomodReviewDto {
    pub id: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub user_id: UserId,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: String,
    pub score: f64,
    pub reason: String,
    pub flags: serde_json::Value,
    pub status: String,
    pub applied_action: Option<String>,
    pub resolved_by_id: Option<String>,
    pub resolved_by_name: Option<String>,
    pub resolved_source: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
    pub voting_deadline: Option<String>,
    pub decided_action: Option<String>,
    pub quorum_met: bool,
    pub decided_at: Option<String>,
    pub incident_count: i32,
    pub cumulative_score: f64,
    pub incidents: serde_json::Value,
    /// True si ce POST a ete agrege dans une carte existante (pas une creation).
    pub merged: bool,
}

impl From<AutomodReview> for AutomodReviewDto {
    fn from(r: AutomodReview) -> Self {
        Self {
            id: r.id.to_string(),
            guild_id: r.guild_id,
            channel_id: r.channel_id,
            message_id: r.message_id,
            user_id: r.user_id,
            user_name: r.user_name,
            content_preview: r.content_preview,
            suggested_action: r.suggested_action,
            score: r.score,
            reason: r.reason,
            flags: r.flags,
            status: r.status,
            applied_action: r.applied_action,
            resolved_by_id: r.resolved_by_id,
            resolved_by_name: r.resolved_by_name,
            resolved_source: r.resolved_source,
            created_at: r.created_at.to_rfc3339(),
            resolved_at: r.resolved_at.map(|d| d.to_rfc3339()),
            voting_deadline: r.voting_deadline.map(|d| d.to_rfc3339()),
            decided_action: r.decided_action,
            quorum_met: r.quorum_met,
            decided_at: r.decided_at.map(|d| d.to_rfc3339()),
            incident_count: r.incident_count,
            cumulative_score: r.cumulative_score,
            incidents: r.incidents,
            merged: false,
        }
    }
}

/// DTO d'un vote individuel.
#[derive(Debug, Serialize)]
pub struct ReviewVoteDto {
    pub voter_id: String,
    pub voter_name: String,
    pub vote_action: String,
}

impl From<sentinel_core::domain::entities::moderation::review::automod::ReviewVote> for ReviewVoteDto {
    fn from(v: sentinel_core::domain::entities::moderation::review::automod::ReviewVote) -> Self {
        Self { voter_id: v.voter_id, voter_name: v.voter_name, vote_action: v.vote_action }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListReviewsQuery {
    pub limit: Option<i64>,
    /// Si true, inclut les reviews resolues. Default false (pending only).
    pub include_resolved: Option<bool>,
}

/// GET /api/automod/{guild_id}/reviews
pub async fn list_reviews(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<ListReviewsQuery>,
) -> Result<Json<Vec<AutomodReviewDto>>, ApiError> {
    validation::validate_guild_id_path(&guild_id).map_err(ApiError)?;
    let limit = params.limit.unwrap_or(100).clamp(1, 500);
    let reviews = if params.include_resolved.unwrap_or(false) {
        state.automod_reviews_uc.list_recent(&guild_id, limit).await?
    } else {
        state.automod_reviews_uc.list_pending(&guild_id, limit).await?
    };
    Ok(Json(reviews.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct CreateReviewBody {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub user_id: UserId,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: String,
    pub score: f64,
    pub reason: String,
    pub flags: Option<serde_json::Value>,
    /// Si fourni (RFC3339), ouvre la review en mode VOTE avec cette echeance.
    pub voting_deadline: Option<String>,
    /// Si true, agrege l'incident dans la carte 'voting' ouverte du meme
    /// utilisateur (anti-flood). Default false (comportement historique).
    pub aggregate: Option<bool>,
}

/// POST /api/automod/reviews
///
/// Endpoint d'ingestion : appele par le bot juste apres avoir poste la
/// carte de review dans le channel Discord. Permet au web de lister les
/// reviews en attente.
pub async fn create_review(
    State(state): State<AppState>,
    Json(body): Json<CreateReviewBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    let suggested = SuggestedAction::from_str(&body.suggested_action).ok_or_else(|| {
        ApiError::from(DomainError::ValidationError(format!(
            "suggested_action invalide : {}",
            body.suggested_action
        )))
    })?;

    let (review, merged) = state
        .automod_reviews_uc
        .create_or_merge(
            NewAutomodReview {
                guild_id: body.guild_id.clone(),
                channel_id: body.channel_id,
                message_id: body.message_id,
                user_id: body.user_id.clone(),
                user_name: body.user_name,
                content_preview: body.content_preview,
                suggested_action: suggested,
                score: body.score,
                reason: body.reason,
                flags: body.flags.unwrap_or(serde_json::json!({})),
                voting_deadline: body
                    .voting_deadline
                    .as_deref()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&chrono::Utc)),
            },
            body.aggregate.unwrap_or(false),
        )
        .await?;

    // Notification web : creation OU mise a jour (agregation) d'une review.
    state.broadcaster.broadcast(
        if merged { "automod_review_updated" } else { "automod_review_created" },
        serde_json::json!({
            "review_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "user_id": &review.user_id,
            "merged": merged,
        }),
    );

    let mut dto: AutomodReviewDto = review.into();
    dto.merged = merged;
    Ok(Json(dto))
}

#[derive(Debug, Deserialize)]
pub struct ResolveReviewBody {
    /// "warn" | "delete" | "mute" | "ban" | "ignore".
    pub applied_action: String,
    pub resolved_by_id: String,
    pub resolved_by_name: String,
    /// "web" (defaut) ou "discord" (finalisation via bouton admin du bot).
    pub source: Option<String>,
}

/// POST /api/automod/reviews/{review_id}/resolve
///
/// Marque la review comme resolue cote DB et publie l'event
/// `automod.review.resolved` avec `actor.source = "web"` pour que le bot
/// edite la carte Discord (greyed-out + footer "via web") et applique
/// l'action Discord (warn/mute/ban/delete) en miroir.
pub async fn resolve_review(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
    Json(body): Json<ResolveReviewBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    let id = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::from(DomainError::ValidationError("review_id invalide".into())))?;

    let source = match body.source.as_deref() {
        Some("discord") => "discord",
        _ => "web",
    };
    let review = state
        .automod_reviews_uc
        .resolve(ResolveAutomodReviewCommand {
            review_id: id,
            applied_action: body.applied_action.clone(),
            resolved_by_id: body.resolved_by_id.clone(),
            resolved_by_name: body.resolved_by_name.clone(),
            resolved_source: source.into(),
        })
        .await?;

    // Event WebSocket + Redis Stream pour le bot listener.
    state.broadcaster.broadcast(
        "automod_review_resolved",
        serde_json::json!({
            "review_id": review.id.to_string(),
            "action_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "user_id": &review.user_id,
            "applied_action": &body.applied_action,
            "actor": {
                "source": source,
                "id": &body.resolved_by_id,
                "name": &body.resolved_by_name,
            },
        }),
    );

    Ok(Json(review.into()))
}

#[derive(Debug, Deserialize)]
pub struct CastVoteBody {
    pub voter_id: String,
    pub voter_name: String,
    /// "warn" | "delete" | "mute" | "ban" | "ignore".
    pub vote_action: String,
}

/// POST /api/automod/reviews/{review_id}/vote
pub async fn vote_review(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
    Json(body): Json<CastVoteBody>,
) -> Result<Json<Vec<ReviewVoteDto>>, ApiError> {
    let id = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::from(DomainError::ValidationError("review_id invalide".into())))?;
    let votes = state
        .automod_reviews_uc
        .cast_vote(crate::ports::inbound::moderation::manage_automod_reviews::CastVoteCommand {
            review_id: id,
            voter_id: body.voter_id.clone(),
            voter_name: body.voter_name.clone(),
            vote_action: body.vote_action.clone(),
        })
        .await?;
    state.broadcaster.broadcast(
        "automod_review_voted",
        serde_json::json!({ "review_id": review_id, "votes": votes.len() }),
    );
    Ok(Json(votes.into_iter().map(Into::into).collect()))
}

/// GET /api/automod/reviews/{review_id}
pub async fn get_review(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    let id = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::from(DomainError::ValidationError("review_id invalide".into())))?;
    match state.automod_reviews_uc.get(id).await? {
        Some(r) => Ok(Json(r.into())),
        None => Err(ApiError::from(DomainError::NotFound(format!("review {review_id} introuvable")))),
    }
}

/// GET /api/automod/reviews/{review_id}/votes
pub async fn list_review_votes(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
) -> Result<Json<Vec<ReviewVoteDto>>, ApiError> {
    let id = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::from(DomainError::ValidationError("review_id invalide".into())))?;
    let votes = state.automod_reviews_uc.list_votes(id).await?;
    Ok(Json(votes.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct DecideReviewBody {
    pub quorum: i64,
    /// "ignore" | "clemente" | "severe".
    pub tie_action: String,
}

/// POST /api/automod/reviews/{review_id}/decide
///
/// Cloture le vote (appele par le worker a l'echeance). Depouille et passe
/// la review en 'decided'. Publie `automod_review_decided` pour que le bot
/// edite la carte et revele le bouton admin de finalisation.
pub async fn decide_review(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
    Json(body): Json<DecideReviewBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    let id = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::from(DomainError::ValidationError("review_id invalide".into())))?;
    let quorum = body.quorum.clamp(1, 100) as usize;
    let (review, tally) = state.automod_reviews_uc.decide(id, quorum, &body.tie_action).await?;
    state.broadcaster.broadcast(
        "automod_review_decided",
        serde_json::json!({
            "review_id": review.id.to_string(),
            "action_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "decided_action": &review.decided_action,
            "quorum_met": tally.quorum_met,
            "total_votes": tally.total_votes,
        }),
    );
    Ok(Json(review.into()))
}

/// DTO d'un salon de discussion lie a une review.
#[derive(Debug, Serialize)]
pub struct DiscussionChannelDto {
    pub id: String,
    pub review_id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub opened_by_id: String,
    pub opened_by_name: String,
    pub created_at: String,
    /// True si ce POST vient de creer le salon (false = il existait deja).
    pub created: bool,
}

impl DiscussionChannelDto {
    fn build(
        d: sentinel_core::domain::entities::moderation::review::automod::DiscussionChannel,
        created: bool,
    ) -> Self {
        Self {
            id: d.id.to_string(),
            review_id: d.review_id.to_string(),
            guild_id: d.guild_id,
            channel_id: d.channel_id,
            opened_by_id: d.opened_by_id,
            opened_by_name: d.opened_by_name,
            created_at: d.created_at.to_rfc3339(),
            created,
        }
    }
}

/// GET /api/automod/reviews/{review_id}/discussion
/// Retourne le salon de discussion existant (ou `null`).
pub async fn get_discussion(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
) -> Result<Json<Option<DiscussionChannelDto>>, ApiError> {
    let id = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::from(DomainError::ValidationError("review_id invalide".into())))?;
    let existing = state.automod_reviews_uc.get_discussion(id).await?;
    Ok(Json(existing.map(|d| DiscussionChannelDto::build(d, false))))
}

#[derive(Debug, Deserialize)]
pub struct OpenDiscussionBody {
    pub guild_id: String,
    pub channel_id: String,
    pub opened_by_id: String,
    pub opened_by_name: String,
    // Faits Discord du demandeur (la decision d'acces est prise par le domaine).
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub has_moderate_members: bool,
    #[serde(default)]
    pub has_manage_messages: bool,
    #[serde(default)]
    pub has_mod_role: bool,
}

/// POST /api/automod/reviews/{review_id}/discussion
/// Enregistre (idempotent) un salon de discussion apres application de la
/// regle d'acces (`can_open_discussion`). `403` si non autorise.
pub async fn open_discussion(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
    Json(body): Json<OpenDiscussionBody>,
) -> Result<Json<DiscussionChannelDto>, ApiError> {
    use sentinel_core::domain::entities::moderation::review::automod::DiscussionRequester;
    use crate::ports::inbound::moderation::manage_automod_reviews::OpenDiscussionCommand;

    let id = Uuid::parse_str(&review_id)
        .map_err(|_| ApiError::from(DomainError::ValidationError("review_id invalide".into())))?;

    let (discussion, created) = state
        .automod_reviews_uc
        .open_discussion(OpenDiscussionCommand {
            review_id: id,
            guild_id: body.guild_id.clone(),
            channel_id: body.channel_id,
            opened_by_id: body.opened_by_id.clone(),
            opened_by_name: body.opened_by_name,
            requester: DiscussionRequester {
                is_admin: body.is_admin,
                has_moderate_members: body.has_moderate_members,
                has_manage_messages: body.has_manage_messages,
                has_mod_role: body.has_mod_role,
            },
        })
        .await?;

    if created {
        state.broadcaster.broadcast(
            "automod_discussion_opened",
            serde_json::json!({
                "review_id": review_id,
                "guild_id": &body.guild_id,
                "channel_id": &discussion.channel_id,
                "opened_by_id": &body.opened_by_id,
            }),
        );
    }

    Ok(Json(DiscussionChannelDto::build(discussion, created)))
}

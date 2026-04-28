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
use crate::domain::entities::moderation::automod_review::AutomodReview;
use crate::domain::entities::moderation::automod_review::NewAutomodReview;
use crate::domain::entities::moderation::automod_review::SuggestedAction;
use crate::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::ports::inbound::moderation::manage_automod_reviews::ResolveAutomodReviewCommand;
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
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub user_id: String,
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
        }
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
    pub guild_id: String,
    pub channel_id: String,
    pub message_id: String,
    pub user_id: String,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: String,
    pub score: f64,
    pub reason: String,
    pub flags: Option<serde_json::Value>,
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

    let review = state
        .automod_reviews_uc
        .create(NewAutomodReview {
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
        })
        .await?;

    // Notification web : la liste des pending reviews vient d'augmenter.
    state.broadcaster.broadcast(
        "automod_review_created",
        serde_json::json!({
            "review_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "user_id": &review.user_id,
        }),
    );

    Ok(Json(review.into()))
}

#[derive(Debug, Deserialize)]
pub struct ResolveReviewBody {
    /// "warn" | "delete" | "mute" | "ban" | "ignore".
    pub applied_action: String,
    pub resolved_by_id: String,
    pub resolved_by_name: String,
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

    let review = state
        .automod_reviews_uc
        .resolve(ResolveAutomodReviewCommand {
            review_id: id,
            applied_action: body.applied_action.clone(),
            resolved_by_id: body.resolved_by_id.clone(),
            resolved_by_name: body.resolved_by_name.clone(),
            resolved_source: "web".into(),
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
                "source": "web",
                "id": &body.resolved_by_id,
                "name": &body.resolved_by_name,
            },
        }),
    );

    Ok(Json(review.into()))
}

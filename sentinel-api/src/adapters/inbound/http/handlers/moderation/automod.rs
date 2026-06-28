//! Handler HTTP du module Automod (Phase 4).
//!
//! Pas de logique metier ici — on reutilise `ManageInfractionsUseCase`
//! (port inbound) avec un filtre `action="detection"`. La page
//! `/automod` cote web consomme ce endpoint pour la timeline des
//! detections automod.

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

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
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<DetectionQuery>,
) -> Result<Json<Vec<InfractionResponseDto>>, ApiError> {

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
    /// Salon de discussion lie a cette review (si ouvert), pour la page web.
    pub discussion_channel_id: Option<String>,
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
            discussion_channel_id: None,
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
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<ListReviewsQuery>,
) -> Result<Json<Vec<AutomodReviewDto>>, ApiError> {
    let limit = params.limit.unwrap_or(100).clamp(1, 500);
    let reviews = if params.include_resolved.unwrap_or(false) {
        state.automod_reviews_uc.list_recent(&guild_id, limit).await?
    } else {
        state.automod_reviews_uc.list_pending(&guild_id, limit).await?
    };
    // Enrichit chaque carte avec son salon de discussion (si ouvert) pour le web.
    let mut dtos: Vec<AutomodReviewDto> = Vec::with_capacity(reviews.len());
    for r in reviews {
        let rid = r.id;
        let mut dto: AutomodReviewDto = r.into();
        if let Ok(Some(d)) = state.automod_reviews_uc.get_discussion(rid).await {
            dto.discussion_channel_id = Some(d.channel_id);
        }
        dtos.push(dto);
    }
    Ok(Json(dtos))
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
    /// Fenetre d'inactivite (minutes) au-dela de laquelle on n'agrege plus dans
    /// une carte existante. Default 60 ; 0 = pas de limite.
    pub aggregate_window_minutes: Option<i64>,
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
            body.aggregate_window_minutes.unwrap_or(60),
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
    // Faits Discord du demandeur (source "discord" uniquement). La regle
    // can_finalize_review est appliquee cote domaine.
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub has_moderate_members: bool,
    #[serde(default)]
    pub has_manage_messages: bool,
    #[serde(default)]
    pub has_mod_role: bool,
    #[serde(default)]
    pub has_admin_role: bool,
}

/// Enregistre la sanction de membre correspondant a une resolution de carte,
/// cote serveur (historique de moderation + escalade), au lieu d'un 2e appel
/// HTTP par le bot. Seules les vraies sanctions de membre sont tracees
/// (prevention/warn/mute/ban) ; "delete"/"ignore" ne sont pas des sanctions.
/// Best-effort : un echec est logge mais ne fait pas echouer la resolution.
async fn log_review_sanction(
    state: &AppState,
    review: &AutomodReview,
    applied_action: &str,
    moderator_id: &str,
    moderator_name: &str,
) {
    use crate::ports::inbound::moderation::manage_moderation::LogModerationCommand;

    if !matches!(applied_action, "prevention" | "warn" | "mute" | "ban") {
        return;
    }

    // Duree du mute depuis la config guild (pour le rappel d'expiration + l'historique).
    let duration = if applied_action == "mute" {
        state
            .bot_config_repo
            .get_config(review.guild_id.as_str(), "automod-bot")
            .await
            .unwrap_or_default()
            .iter()
            .find(|e| e.config_key == "mute_duration_secs")
            .and_then(|e| e.config_value.parse::<u64>().ok())
    } else {
        None
    };

    let cmd = LogModerationCommand {
        guild_id: review.guild_id.clone(),
        channel_id: review.channel_id.clone(),
        moderator_id: moderator_id.to_string(),
        moderator_name: moderator_name.to_string(),
        target_id: review.user_id.as_str().to_string(),
        target_name: review.user_name.clone(),
        action_type: applied_action.to_string(),
        reason: "Sanction validee via carte AutoMod".to_string(),
        gravity: if applied_action == "warn" { Some("medium".to_string()) } else { None },
        duration,
    };
    let logged = match state.moderation_uc.log_action_with_strike(cmd).await {
        Ok(l) => l,
        Err(e) => {
            // Compteur "logs manquants" : si non nul en prod, on active l'outbox
            // (cf. ADR / CR revue moderation). Mesure la fenetre resolve->log.
            metrics::counter!("automod_sanction_log_total", "result" => "error").increment(1);
            tracing::error!(error = %e, review_id = %review.id, action = %applied_action, "Echec log sanction (resolve) cote serveur");
            return;
        }
    };
    metrics::counter!("automod_sanction_log_total", "result" => "ok").increment(1);

    // Memes broadcasts que l'endpoint /api/moderation/actions, pour que le
    // journal web et les notifications de strike restent a jour.
    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": applied_action,
            "target_id": review.user_id.as_str(),
            "target_name": &review.user_name,
            "moderator_name": moderator_name,
            "reason": "Sanction validee via carte AutoMod",
            "guild_id": review.guild_id.as_str(),
        }),
    );
    if let Some(sr) = &logged.strike {
        if sr.should_trigger_escalation_broadcast() {
            state.broadcaster.broadcast(
                "strike_added",
                serde_json::json!({
                    "guild_id": review.guild_id.as_str(),
                    "user_id": review.user_id.as_str(),
                    "active_count": sr.active_count,
                    "escalation_action": sr.escalation_action,
                    "escalation_duration": sr.escalation_duration,
                }),
            );
        }
    }
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
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;

    let source = match body.source.as_deref() {
        Some("discord") => "discord",
        _ => "web",
    };
    // Faits du demandeur seulement pour la finalisation Discord (la source
    // "web" est autorisee par guild_auth en amont -> requester None).
    let requester = if source == "discord" {
        Some(sentinel_core::domain::entities::moderation::review::automod::ModeratorFacts {
            is_admin: body.is_admin,
            has_moderate_members: body.has_moderate_members,
            has_manage_messages: body.has_manage_messages,
            has_mod_role: body.has_mod_role,
            has_admin_role: body.has_admin_role,
        })
    } else {
        None
    };
    let review = state
        .automod_reviews_uc
        .resolve(ResolveAutomodReviewCommand {
            review_id: id,
            applied_action: body.applied_action.clone(),
            resolved_by_id: body.resolved_by_id.clone(),
            resolved_by_name: body.resolved_by_name.clone(),
            resolved_source: source.into(),
            requester,
        })
        .await?;

    // Tracabilite : on enregistre la sanction de membre cote serveur, dans la
    // meme requete que la resolution (le bot n'a plus a faire un 2e appel
    // HTTP -> plus de fenetre "resolu mais non logge" cote bot).
    log_review_sanction(
        &state, &review, &body.applied_action, &body.resolved_by_id, &body.resolved_by_name,
    )
    .await;

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
pub struct CloseIgnoreBody {
    pub actor_id: String,
    pub actor_name: String,
    /// "web" (defaut) ou "discord".
    pub source: Option<String>,
    // Faits Discord du demandeur (source "discord"). Regle is_moderator cote domaine.
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub has_moderate_members: bool,
    #[serde(default)]
    pub has_manage_messages: bool,
    #[serde(default)]
    pub has_mod_role: bool,
    #[serde(default)]
    pub has_admin_role: bool,
}

fn discord_facts_or_none(
    source: &str,
    is_admin: bool,
    has_moderate_members: bool,
    has_manage_messages: bool,
    has_mod_role: bool,
    has_admin_role: bool,
) -> Option<sentinel_core::domain::entities::moderation::review::automod::ModeratorFacts> {
    if source == "discord" {
        Some(sentinel_core::domain::entities::moderation::review::automod::ModeratorFacts {
            is_admin,
            has_moderate_members,
            has_manage_messages,
            has_mod_role,
            has_admin_role,
        })
    } else {
        None
    }
}

/// POST /api/automod/reviews/{review_id}/ignore
/// Clore immediatement le dossier en "ignore" (tout moderateur).
pub async fn ignore_review(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
    Json(body): Json<CloseIgnoreBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    use crate::ports::inbound::moderation::manage_automod_reviews::CloseIgnoredCommand;
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let source = match body.source.as_deref() {
        Some("discord") => "discord",
        _ => "web",
    };
    let requester = discord_facts_or_none(
        source, body.is_admin, body.has_moderate_members, body.has_manage_messages,
        body.has_mod_role, body.has_admin_role,
    );
    let review = state
        .automod_reviews_uc
        .close_ignored(CloseIgnoredCommand {
            review_id: id,
            actor_id: body.actor_id.clone(),
            actor_name: body.actor_name.clone(),
            source: source.into(),
            requester,
        })
        .await?;

    state.broadcaster.broadcast(
        "automod_review_resolved",
        serde_json::json!({
            "review_id": review.id.to_string(),
            "action_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "user_id": &review.user_id,
            "applied_action": "ignore",
            "actor": { "source": source, "id": &body.actor_id, "name": &body.actor_name },
        }),
    );
    Ok(Json(review.into()))
}

#[derive(Debug, Deserialize)]
pub struct ReopenBody {
    pub actor_id: String,
    pub actor_name: String,
    /// Duree (heures) de la nouvelle fenetre de vote (defaut 72).
    #[serde(default)]
    pub deadline_hours: Option<i64>,
    pub source: Option<String>,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub has_moderate_members: bool,
    #[serde(default)]
    pub has_manage_messages: bool,
    #[serde(default)]
    pub has_mod_role: bool,
    #[serde(default)]
    pub has_admin_role: bool,
}

/// POST /api/automod/reviews/{review_id}/reopen
/// Rouvrir un dossier resolu/ignore -> repasse en vote (tout moderateur).
pub async fn reopen_review(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
    Json(body): Json<ReopenBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    use crate::ports::inbound::moderation::manage_automod_reviews::ReopenReviewCommand;
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let source = match body.source.as_deref() {
        Some("discord") => "discord",
        _ => "web",
    };
    let requester = discord_facts_or_none(
        source, body.is_admin, body.has_moderate_members, body.has_manage_messages,
        body.has_mod_role, body.has_admin_role,
    );
    let review = state
        .automod_reviews_uc
        .reopen(ReopenReviewCommand {
            review_id: id,
            actor_id: body.actor_id.clone(),
            actor_name: body.actor_name.clone(),
            deadline_hours: body.deadline_hours.unwrap_or(72),
            source: source.into(),
            requester,
        })
        .await?;

    state.broadcaster.broadcast(
        "automod_review_reopened",
        serde_json::json!({
            "review_id": review.id.to_string(),
            "action_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "user_id": &review.user_id,
            "actor": { "source": source, "id": &body.actor_id, "name": &body.actor_name },
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
    // Faits Discord du votant ; la regle is_moderator est appliquee cote domaine.
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub has_moderate_members: bool,
    #[serde(default)]
    pub has_manage_messages: bool,
    #[serde(default)]
    pub has_mod_role: bool,
}

/// POST /api/automod/reviews/{review_id}/vote
pub async fn vote_review(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
    Json(body): Json<CastVoteBody>,
) -> Result<Json<Vec<ReviewVoteDto>>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let votes = state
        .automod_reviews_uc
        .cast_vote(crate::ports::inbound::moderation::manage_automod_reviews::CastVoteCommand {
            review_id: id,
            voter_id: body.voter_id.clone(),
            voter_name: body.voter_name.clone(),
            vote_action: body.vote_action.clone(),
            requester: sentinel_core::domain::entities::moderation::review::automod::ModeratorFacts {
                is_admin: body.is_admin,
                has_moderate_members: body.has_moderate_members,
                has_manage_messages: body.has_manage_messages,
                has_mod_role: body.has_mod_role,
                has_admin_role: false,
            },
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
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    match state.automod_reviews_uc.get(id).await? {
        Some(r) => {
            let rid = r.id;
            let mut dto: AutomodReviewDto = r.into();
            if let Ok(Some(d)) = state.automod_reviews_uc.get_discussion(rid).await {
                dto.discussion_channel_id = Some(d.channel_id);
            }
            Ok(Json(dto))
        }
        None => Err(ApiError::from(DomainError::NotFound(format!("review {review_id} introuvable")))),
    }
}

/// GET /api/automod/reviews/{review_id}/votes
pub async fn list_review_votes(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
) -> Result<Json<Vec<ReviewVoteDto>>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
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
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
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
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let existing = state.automod_reviews_uc.get_discussion(id).await?;
    Ok(Json(existing.map(|d| DiscussionChannelDto::build(d, false))))
}

/// DELETE /api/automod/reviews/{review_id}/discussion
/// Purge l'enregistrement du salon (le salon Discord a ete supprime a la
/// main) afin de pouvoir en rouvrir un neuf. Idempotent.
pub async fn delete_discussion(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    state.automod_reviews_uc.delete_discussion(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
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
    use sentinel_core::domain::entities::moderation::review::automod::ModeratorFacts;
    use crate::ports::inbound::moderation::manage_automod_reviews::OpenDiscussionCommand;

    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;

    let (discussion, created) = state
        .automod_reviews_uc
        .open_discussion(OpenDiscussionCommand {
            review_id: id,
            guild_id: body.guild_id.clone(),
            channel_id: body.channel_id,
            opened_by_id: body.opened_by_id.clone(),
            opened_by_name: body.opened_by_name,
            requester: ModeratorFacts {
                is_admin: body.is_admin,
                has_moderate_members: body.has_moderate_members,
                has_manage_messages: body.has_manage_messages,
                has_mod_role: body.has_mod_role,
                has_admin_role: false,
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

// ── Transcript du salon de discussion (trace persistante) ──

#[derive(Debug, Deserialize)]
pub struct DiscussionMessageIn {
    pub discord_message_id: String,
    pub author_id: String,
    #[serde(default)]
    pub author_name: String,
    #[serde(default)]
    pub author_is_bot: bool,
    #[serde(default)]
    pub content: String,
    /// RFC3339.
    pub sent_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AppendDiscussionMessagesBody {
    pub messages: Vec<DiscussionMessageIn>,
}

#[derive(Debug, Serialize)]
pub struct DiscussionMessageDto {
    pub discord_message_id: String,
    pub author_id: String,
    pub author_name: String,
    pub author_is_bot: bool,
    pub content: String,
    pub sent_at: String,
}

/// POST /api/automod/reviews/{review_id}/discussion/messages
/// Persiste un lot de messages du salon (appele par le bot a l'archivage).
pub async fn append_discussion_messages(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
    Json(body): Json<AppendDiscussionMessagesBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use sentinel_core::domain::entities::moderation::review::automod::DiscussionMessage;
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;

    let messages: Vec<DiscussionMessage> = body
        .messages
        .into_iter()
        .filter_map(|m| {
            let sent_at = chrono::DateTime::parse_from_rfc3339(&m.sent_at)
                .ok()?
                .with_timezone(&chrono::Utc);
            Some(DiscussionMessage {
                review_id: id,
                discord_message_id: m.discord_message_id,
                author_id: m.author_id,
                author_name: m.author_name,
                author_is_bot: m.author_is_bot,
                content: m.content,
                sent_at,
            })
        })
        .collect();

    let inserted = state.automod_reviews_uc.append_discussion_messages(messages).await?;
    Ok(Json(serde_json::json!({ "inserted": inserted })))
}

#[derive(Debug, Deserialize)]
pub struct CleanupCardsQuery {
    /// Age minimum (jours) d'une carte close pour etre supprimee. Defaut 30.
    pub days: Option<i64>,
}

/// POST /api/automod/cleanup-expired-cards — appele par le worker (24h).
/// Pour chaque carte de review CLOSE (applied|ignored) resolue depuis plus de
/// `days` jours et encore mappee a un message Discord : broadcast un event
/// `automod_card_expired` (le bot supprime le message) et retire le mapping.
/// La review + le transcript restent en DB (trace web conservee).
pub async fn cleanup_expired_cards(
    State(state): State<AppState>,
    Query(q): Query<CleanupCardsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Le use case trouve les cartes expirees et retire leur mapping ; le
    // handler ne fait que diffuser l'event d'expiration (le bot supprime le
    // message Discord correspondant).
    let cards = state
        .automod_reviews_uc
        .expired_review_cards(q.days.unwrap_or(30), 200)
        .await?;

    let count = cards.len() as u32;
    for c in &cards {
        state.broadcaster.broadcast(
            "automod_card_expired",
            serde_json::json!({
                "action_id": c.action_id.to_string(),
                "channel_id": c.channel_id,
                "message_id": c.message_id,
            }),
        );
    }
    Ok(Json(serde_json::json!({ "expired": count })))
}

/// GET /api/automod/{guild_id}/reviews/by-message/{message_id}
/// Retrouve la review associee a un message Discord (pour retrouver le
/// review_id depuis une carte 1-clic dont les boutons ne le portent pas).
pub async fn find_review_by_message(
    State(state): State<AppState>,
    Path((guild_id, message_id)): Path<(String, String)>,
) -> Result<Json<Option<AutomodReviewDto>>, ApiError> {
    let review = state
        .automod_reviews_uc
        .find_by_message_id(&guild_id, &message_id)
        .await?;
    Ok(Json(review.map(Into::into)))
}

/// GET /api/automod/reviews/{review_id}/discussion/messages
/// Liste le transcript (trace) pour affichage web.
pub async fn list_discussion_messages(
    State(state): State<AppState>,
    Path(review_id): Path<String>,
) -> Result<Json<Vec<DiscussionMessageDto>>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let msgs = state.automod_reviews_uc.list_discussion_messages(id).await?;
    let dtos = msgs
        .into_iter()
        .map(|m| DiscussionMessageDto {
            discord_message_id: m.discord_message_id,
            author_id: m.author_id,
            author_name: m.author_name,
            author_is_bot: m.author_is_bot,
            content: m.content,
            sent_at: m.sent_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(dtos))
}

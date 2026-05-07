use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::moderation::actions::BanEntryDto;
use crate::adapters::inbound::http::dto::moderation::actions::LogActionDto;
use crate::adapters::inbound::http::dto::moderation::actions::ModerationActionResponseDto;
use crate::adapters::inbound::http::dto::moderation::actions::UserHistoryDto;
use tracing::warn;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::rbac::check_role;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use sentinel_core::domain::enums::system::role::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::adapters::inbound::http::validation;
use sentinel_core::domain::errors::DomainError;
use crate::ports::inbound::moderation::manage_reminders::CreateReminderCommand;
use sentinel_core::domain::entities::system::discord_ids::UserId;
use sentinel_core::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Deserialize)]
pub struct BansQuery {
    pub guild_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// POST /api/moderation/actions — enregistrer une action de modération
pub async fn log_action(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<LogActionDto>,
) -> Result<Json<ModerationActionResponseDto>, ApiError> {
    // Validation
    validation::validate_moderation_action(
        &dto.guild_id, &dto.moderator_id, &dto.target_id, &dto.reason, &dto.action_type,
    ).map_err(ApiError)?;

    // Phase 7B — Gate RBAC (pass-through pour les appels bot/internal sans token Discord).
    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Moderator,
        "moderator+ requis pour enregistrer une action de moderation",
    )
    .await?;

    let action_type = dto.action_type.clone();
    let target_name = dto.target_name.clone();
    let moderator_name = dto.moderator_name.clone();
    let reason = dto.reason.clone();

    let guild_id = dto.guild_id.clone();
    let target_id = dto.target_id.clone();
    let moderator_id = dto.moderator_id.clone();
    let duration = dto.duration;

    let command = dto.into();
    // Orchestration atomique (action + strike) dans le service.
    let logged = state.moderation_uc.log_action_with_strike(command).await?;
    let action = logged.action;
    let strike_result = logged.strike;

    let mut dto = ModerationActionResponseDto::from(action);
    if let Some(ref sr) = strike_result {
        dto.strikes_count = Some(sr.active_count);
        dto.escalation_action = sr.escalation_action.clone();
        dto.escalation_duration = sr.escalation_duration;
    }

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": action_type,
            "target_id": target_id,
            "target_name": target_name,
            "moderator_name": moderator_name,
            "reason": reason,
            "guild_id": guild_id,
        }),
    );

    if let Some(ref sr) = strike_result {
        if sr.should_trigger_escalation_broadcast() {
            state.broadcaster.broadcast(
                "strike_added",
                serde_json::json!({
                    "guild_id": guild_id,
                    "user_id": target_id,
                    "active_count": sr.active_count,
                    "escalation_action": sr.escalation_action,
                    "escalation_duration": sr.escalation_duration,
                }),
            );
        }
    }

    // Auto-create reminder for temporary sanctions (regle metier : voir
    // `ModerationActionType::is_temporary` dans domain/value_objects).
    if sentinel_core::domain::enums::moderation::moderation_action_type::ModerationActionType::is_temporary_str(&action_type) {
        if let Some(dur) = duration {
            let action_uuid = match dto.id.parse() {
                Ok(uuid) => uuid,
                Err(e) => {
                    warn!(error = %e, id = %dto.id, "UUID action invalide pour rappel, utilisation UUID nil");
                    uuid::Uuid::nil()
                }
            };
            if let Err(e) = state.reminders_uc.create_reminder(CreateReminderCommand {
                guild_id: guild_id.clone().into(),
                moderator_id,
                moderator_name: moderator_name.clone(),
                target_id: target_id.clone(),
                target_name: target_name.clone(),
                action_type: action_type.clone(),
                reason: reason.clone(),
                action_id: action_uuid,
                duration_secs: dur,
                remind_before_secs: state.bot_config_reminder_advance_secs(&guild_id).await,
            }).await {
                // Niveau ERROR (pas warn) : une sanction temporaire sans
                // rappel = durée perpétuelle jusqu'à intervention manuelle.
                // Broadcast aussi pour alerting desktop.
                tracing::error!(
                    error = %e,
                    guild_id = %guild_id,
                    target_id = %target_id,
                    action_id = %dto.id,
                    duration_secs = dur,
                    "INCOHERENCE : sanction temporaire sans reminder — intervention manuelle requise"
                );
                state.broadcaster.broadcast(
                    "reminder_creation_failed",
                    serde_json::json!({
                        "version": "1",
                        "guild_id": guild_id,
                        "target_id": target_id,
                        "action_id": dto.id,
                        "action_type": action_type,
                        "error": e.to_string(),
                    }),
                );
            }
        }
    }

    Ok(Json(dto))
}

#[derive(Debug, Deserialize)]
pub struct ExecuteBanDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub reason: String,
    /// Phase 1 sync (cf. SYNC_DISCORD_WEB_DESIGN.md) : si fourni, l API
    /// publie un event `moderation.ban.executed` avec cet `action_id`,
    /// permettant au bot d editer le message Discord correspondant.
    #[serde(default)]
    pub action_id: Option<uuid::Uuid>,
}

/// POST /api/moderation/execute-ban — execute un ban Discord + log l'action
pub async fn execute_ban(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<ExecuteBanDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_reason(&dto.reason).map_err(ApiError)?;

    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Moderator,
        "moderator+ requis pour executer un ban",
    )
    .await?;

    state
        .discord_api
        .ban_user(&dto.guild_id, &dto.user_id, &dto.reason)
        .await
        .map_err(ApiError)?;

    let reason = dto.reason.clone();

    let command = crate::ports::inbound::moderation::manage_moderation::LogModerationCommand {
        guild_id: dto.guild_id.clone(),
        channel_id: String::new().into(),
        moderator_id: "desktop".into(),
        moderator_name: "Desktop App".into(),
        target_id: dto.user_id.clone().into(),
        target_name: dto.user_id.clone().into(),
        action_type: "ban_permanent".into(),
        reason: dto.reason,
        gravity: None,
        duration: None,
    };
    state.moderation_uc.log_action(command).await?;

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": "ban_permanent",
            "target_id": &dto.user_id,
            "target_name": &dto.user_id,
            "moderator_name": "Desktop App",
            "guild_id": &dto.guild_id,
            "reason": &reason,
        }),
    );

    // Phase 1 sync : event dedie pour le bot et le web (refresh + edit
    // message Discord). Format aligne sur SYNC_DISCORD_WEB_DESIGN.md.
    if let Some(action_id) = dto.action_id {
        state.broadcaster.broadcast(
            "moderation.ban.executed",
            serde_json::json!({
                "action_id": action_id,
                "guild_id": &dto.guild_id,
                "target_id": &dto.user_id,
                "actor": { "user_id": "desktop", "source": "web" },
                "reason": &reason,
            }),
        );
    }

    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct ExecuteMuteDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub reason: String,
    /// Duree du timeout en secondes. Defaut : 1h. Max : 28 jours (clamp cote Discord).
    #[serde(default)]
    pub duration: Option<u64>,
    /// Nom d'affichage optionnel (stocke dans moderation_actions.target_name).
    #[serde(default)]
    pub target_name: Option<String>,
}

/// POST /api/moderation/execute-mute — applique un timeout Discord + log l'action
pub async fn execute_mute(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<ExecuteMuteDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_reason(&dto.reason).map_err(ApiError)?;

    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Moderator,
        "moderator+ requis pour executer un mute",
    )
    .await?;

    let duration = sentinel_core::domain::entities::moderation::review::manual::resolve_mute_duration(dto.duration);
    state
        .discord_api
        .apply_timeout(&dto.guild_id, &dto.user_id, duration)
        .await
        .map_err(ApiError)?;

    let target_name = dto.target_name.unwrap_or_else(|| dto.user_id.clone().into());
    let command = crate::ports::inbound::moderation::manage_moderation::LogModerationCommand {
        guild_id: dto.guild_id.clone(),
        channel_id: String::new().into(),
        moderator_id: "web-panel".into(),
        moderator_name: "Web Admin".into(),
        target_id: dto.user_id.clone().into(),
        target_name: target_name.clone(),
        action_type: "mute".into(),
        reason: dto.reason.clone(),
        gravity: None,
        duration: Some(duration),
    };
    state.moderation_uc.log_action(command).await?;

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": "mute",
            "target_id": &dto.user_id,
            "target_name": &target_name,
            "moderator_name": "Web Admin",
            "guild_id": &dto.guild_id,
            "reason": &dto.reason,
            "duration": duration,
        }),
    );

    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct ExecuteUnbanDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
}

/// POST /api/moderation/execute-unban — debannir un utilisateur Discord
pub async fn execute_unban(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<ExecuteUnbanDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_guild_user_path(&dto.guild_id, &dto.user_id).map_err(ApiError)?;

    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Moderator,
        "moderator+ requis pour deban un user",
    )
    .await?;

    state
        .discord_api
        .unban_user(&dto.guild_id, &dto.user_id)
        .await
        .map_err(ApiError)?;

    let target_id = dto.user_id.clone();
    let guild_id = dto.guild_id.clone();

    let command = crate::ports::inbound::moderation::manage_moderation::LogModerationCommand {
        guild_id: dto.guild_id,
        channel_id: String::new().into(),
        moderator_id: "desktop".into(),
        moderator_name: "Desktop App".into(),
        target_id: target_id.clone().into(),
        target_name: target_id.clone().into(),
        action_type: "unban".into(),
        reason: "Deban depuis le desktop".into(),
        gravity: None,
        duration: None,
    };
    state
        .moderation_uc
        .delete_bans_for_user(&guild_id, &target_id)
        .await?;
    state.moderation_uc.log_action(command).await?;

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": "unban",
            "target_id": &target_id,
            "moderator_name": "Desktop App",
            "guild_id": &guild_id,
        }),
    );

    Ok(ok_response())
}

/// GET /api/moderation/bans
pub async fn list_bans(
    State(state): State<AppState>,
    Query(params): Query<BansQuery>,
) -> Result<Json<Vec<BanEntryDto>>, ApiError> {
    // Validation
    validation::validate_optional_discord_id("guild_id", &params.guild_id).map_err(ApiError)?;
    validation::validate_pagination(params.limit, params.offset).map_err(ApiError)?;

    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 500);
    let offset = crate::adapters::inbound::http::helpers::normalize_offset(params.offset);
    let bans = state
        .moderation_uc
        .list_bans(params.guild_id.as_deref(), limit, offset)
        .await?;
    Ok(map_to_dtos(bans))
}

/// GET /api/moderation/history/{guild_id}/{user_id}
pub async fn get_history(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<UserHistoryDto>, ApiError> {
    // Validation
    validation::validate_guild_user_path(&guild_id, &user_id).map_err(ApiError)?;

    let history = state
        .moderation_uc
        .get_history(&guild_id, &user_id)
        .await?;
    Ok(single_dto(history))
}

/// MOD #2 — POST /api/moderation/evidence
///
/// Attache une preuve (URL + description optionnelle) a une action de moderation
/// existante. La FK assure qu'on ne peut pas attacher a une action inconnue.
#[derive(Debug, serde::Deserialize)]
pub struct AddEvidenceDto {
    pub action_id: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    pub uploaded_by: String,
    pub uploaded_by_name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct EvidenceEntryDto {
    pub id: String,
    pub action_id: String,
    pub url: String,
    pub description: Option<String>,
    pub uploaded_by: String,
    pub uploaded_by_name: String,
    pub uploaded_at: String,
}

pub async fn add_evidence(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<AddEvidenceDto>,
) -> Result<Json<EvidenceEntryDto>, ApiError> {
    // Pour gater RBAC on a besoin du guild_id : on le recupere via l'action liee.
    if rbac.is_some() {
        if let Ok(action_uuid) = uuid::Uuid::parse_str(&dto.action_id) {
            let gid: Option<(String,)> = sqlx::query_as(
                "SELECT guild_id FROM moderation_actions WHERE id = $1",
            )
            .bind(action_uuid)
            .fetch_optional(&state.pg_pool)
            .await
            .map_err(|e| ApiError(DomainError::Internal(format!("fetch action guild_id: {e}"))))?;
            if let Some((guild_id,)) = gid {
                check_role_for_guild(
                    &state,
                    &rbac,
                    &guild_id,
                    Role::Moderator,
                    "moderator+ requis pour attacher une preuve",
                )
                .await?;
            }
        }
    }
    // Validation URL — regle metier dans `domain/entities/moderation_review.rs`.
    sentinel_core::domain::entities::moderation::review::manual::validate_evidence_url(&dto.url)
        .map_err(|m| ApiError(sentinel_core::domain::errors::DomainError::ValidationError(m.into())))?;
    let action_uuid = uuid::Uuid::parse_str(&dto.action_id).map_err(|_| {
        ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
            "action_id invalide".into(),
        ))
    })?;
    validation::validate_discord_id("uploaded_by", &dto.uploaded_by).map_err(ApiError)?;
    let description = dto.description.as_deref().map(sentinel_core::domain::entities::moderation::review::manual::truncate_review_text);

    let entry = state.evidence_repo
        .add(action_uuid, &dto.url, description.as_deref(), &dto.uploaded_by, &dto.uploaded_by_name)
        .await?;

    Ok(Json(EvidenceEntryDto {
        id: entry.id.to_string(),
        action_id: dto.action_id,
        url: entry.url,
        description: entry.description,
        uploaded_by: dto.uploaded_by,
        uploaded_by_name: dto.uploaded_by_name,
        uploaded_at: entry.uploaded_at.to_rfc3339(),
    }))
}

/// MOD #2 — GET /api/moderation/evidence/{action_id}
///
/// Liste les preuves attachees a une action.
pub async fn list_evidence(
    State(state): State<AppState>,
    Path(action_id): Path<String>,
) -> Result<Json<Vec<EvidenceEntryDto>>, ApiError> {
    let action_uuid = uuid::Uuid::parse_str(&action_id).map_err(|_| {
        ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
            "action_id invalide".into(),
        ))
    })?;

    let entries = state.evidence_repo.list(action_uuid).await?;
    let dtos = entries
        .into_iter()
        .map(|e| EvidenceEntryDto {
            id: e.id.to_string(),
            action_id: action_id.clone(),
            url: e.url,
            description: e.description,
            uploaded_by: e.uploaded_by,
            uploaded_by_name: e.uploaded_by_name,
            uploaded_at: e.uploaded_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(dtos))
}

/// MOD #3 — POST /api/moderation/review
#[derive(Debug, serde::Deserialize)]
pub struct AddReviewDto {
    pub action_id: String,
    pub guild_id: GuildId,
    pub added_by: String,
    pub added_by_name: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ReviewQueueEntryDto {
    pub id: String,
    pub action_id: String,
    pub guild_id: GuildId,
    pub added_by: String,
    pub added_by_name: String,
    pub reason: Option<String>,
    pub status: String,
    pub reviewer_id: Option<String>,
    pub reviewer_name: Option<String>,
    pub reviewer_notes: Option<String>,
    pub added_at: String,
    pub resolved_at: Option<String>,
    // Enrichissement : infos de l'action liee
    pub action_type: Option<String>,
    pub target_name: Option<String>,
    pub action_reason: Option<String>,
}

fn review_entry_to_dto(e: crate::ports::outbound::moderation::review_repository::ReviewEntry) -> ReviewQueueEntryDto {
    ReviewQueueEntryDto {
        id: e.id.to_string(),
        action_id: e.action_id.to_string(),
        guild_id: e.guild_id,
        added_by: e.added_by,
        added_by_name: e.added_by_name,
        reason: e.reason,
        status: e.status,
        reviewer_id: e.reviewer_id,
        reviewer_name: e.reviewer_name,
        reviewer_notes: e.reviewer_notes,
        added_at: e.added_at.to_rfc3339(),
        resolved_at: e.resolved_at.map(|d| d.to_rfc3339()),
        action_type: e.action_type,
        target_name: e.target_name,
        action_reason: e.action_reason,
    }
}

pub async fn add_review(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Json(dto): Json<AddReviewDto>,
) -> Result<Json<ReviewQueueEntryDto>, ApiError> {
    let action_uuid = uuid::Uuid::parse_str(&dto.action_id).map_err(|_| {
        ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
            "action_id invalide".into(),
        ))
    })?;
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("added_by", &dto.added_by).map_err(ApiError)?;

    check_role_for_guild(
        &state,
        &rbac,
        &dto.guild_id,
        Role::Moderator,
        "moderator+ requis pour ajouter une review",
    )
    .await?;
    let reason = dto.reason.as_deref().map(sentinel_core::domain::entities::moderation::review::manual::truncate_review_text);

    let entry = state.review_repo
        .add(action_uuid, &dto.guild_id, &dto.added_by, &dto.added_by_name, reason.as_deref())
        .await?;

    Ok(Json(review_entry_to_dto(entry)))
}

/// MOD #3 — GET /api/moderation/review/{guild_id}/pending
///
/// Liste les reviews en attente pour une guild, enrichies avec les infos de
/// l'action de moderation liee (JOIN avec moderation_actions).
pub async fn list_pending_reviews(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<ReviewQueueEntryDto>>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    check_role(&rbac, Role::Moderator, "moderator+ requis pour lister les reviews")?;

    let entries = state.review_repo.list_pending(&guild_id).await?;
    Ok(Json(entries.into_iter().map(review_entry_to_dto).collect()))
}

/// MOD #3 — PATCH /api/moderation/review/{id}/resolve
#[derive(Debug, serde::Deserialize)]
pub struct ResolveReviewDto {
    pub status: String,
    pub reviewer_id: String,
    pub reviewer_name: String,
    #[serde(default)]
    pub reviewer_notes: Option<String>,
}

pub async fn resolve_review(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
    Json(dto): Json<ResolveReviewDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let review_uuid = uuid::Uuid::parse_str(&id).map_err(|_| {
        ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
            "id invalide".into(),
        ))
    })?;

    // RBAC via le repo.
    if rbac.is_some() {
        if let Some(guild_id) = state.review_repo.get_guild_id(review_uuid).await? {
            check_role_for_guild(&state, &rbac, &guild_id, Role::Moderator, "moderator+ requis pour resoudre une review").await?;
        }
    }

    if !sentinel_core::domain::entities::moderation::review::manual::is_valid_review_status(&dto.status) {
        return Err(ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
            "status doit etre approved/rejected/changed".into(),
        )));
    }
    validation::validate_discord_id("reviewer_id", &dto.reviewer_id).map_err(ApiError)?;
    let notes = dto.reviewer_notes.as_deref().map(sentinel_core::domain::entities::moderation::review::manual::truncate_review_text);

    let resolved = state.review_repo
        .resolve(review_uuid, &dto.reviewer_id, &dto.reviewer_name, notes.as_deref(), &dto.status)
        .await?;

    if !resolved {
        return Err(ApiError(sentinel_core::domain::errors::DomainError::NotFound(
            "review introuvable ou deja resolue".into(),
        )));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// MOD #7 — GET /api/moderation/modstats/{guild_id}
///
/// Agrege les actions de moderation par moderateur sur les 30 derniers jours.
/// Retourne le top 20 classe par nombre total d'actions decroissant.
///
/// Approche pragmatique : sqlx direct (pas de use-case), read-only, aggregation simple.
pub async fn get_modstats(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(guild_id): Path<String>,
    Query(params): Query<TrendQuery>,
) -> Result<Json<Vec<crate::adapters::inbound::http::dto::moderation::actions::ModStatsEntryDto>>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    check_role(&rbac, Role::Moderator, "moderator+ requis pour voir les stats de moderation")?;
    let days = params.days.unwrap_or(30).clamp(1, 90);

    #[derive(sqlx::FromRow)]
    struct StatsRow {
        moderator_id: String,
        moderator_name: String,
        total: i64,
        warns: i64,
        mutes: i64,
        bans: i64,
        kicks: i64,
    }

    // Phase 4 : on lit depuis `audit_logs` (event_type='mod_*') et plus
    // depuis `moderation_actions` qui n'est plus alimentee.
    let sql = format!(
        "SELECT \
            actor_id AS moderator_id, \
            MAX(actor_name) AS moderator_name, \
            COUNT(*) AS total, \
            COUNT(*) FILTER (WHERE event_type = 'mod_warn') AS warns, \
            COUNT(*) FILTER (WHERE event_type IN ('mod_mute_temp','mod_mute_permanent','mod_mute')) AS mutes, \
            COUNT(*) FILTER (WHERE event_type IN ('mod_ban_temp','mod_ban_permanent','mod_ban')) AS bans, \
            COUNT(*) FILTER (WHERE event_type = 'mod_kick') AS kicks \
         FROM audit_logs \
         WHERE guild_id = $1 \
           AND event_type LIKE 'mod_%' \
           AND event_type NOT IN ('mod_unban','mod_unmute') \
           AND actor_id IS NOT NULL \
           AND created_at >= NOW() - INTERVAL '{days} days' \
         GROUP BY actor_id \
         ORDER BY total DESC \
         LIMIT 20"
    );
    let rows: Vec<StatsRow> = sqlx::query_as::<_, StatsRow>(&sql)
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError(sentinel_core::domain::errors::DomainError::Internal(format!("modstats query: {e}"))))?;

    let dtos = rows
        .into_iter()
        .map(
            |r| crate::adapters::inbound::http::dto::moderation::actions::ModStatsEntryDto {
                moderator_id: r.moderator_id,
                moderator_name: r.moderator_name,
                total: r.total,
                warns: r.warns,
                mutes: r.mutes,
                bans: r.bans,
                kicks: r.kicks,
            },
        )
        .collect();

    Ok(Json(dtos))
}

/// GET /api/moderation/modstats/{guild_id}/trend?days=30
///
/// Retourne les actions de moderation agregees par jour sur les N derniers
/// jours (default 30, max 90). Lecture depuis `audit_logs` comme modstats.
/// Utilise pour la courbe "Tendance moderation" sur la page web /modstats.
pub async fn get_modstats_trend(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(guild_id): Path<String>,
    Query(params): Query<TrendQuery>,
) -> Result<Json<Vec<ModstatsTrendDayDto>>, ApiError> {
    validation::validate_discord_id("guild_id", &guild_id).map_err(ApiError)?;
    check_role(&rbac, Role::Moderator, "moderator+ requis pour voir les stats de moderation")?;

    let days = params.days.unwrap_or(30).clamp(1, 90);

    #[derive(sqlx::FromRow)]
    struct TrendRow {
        day: chrono::NaiveDate,
        warns: i64,
        mutes: i64,
        bans: i64,
        kicks: i64,
    }

    // generate_series garantit que tous les jours apparaissent meme s'il
    // n'y a eu aucune action ce jour-la (sinon la courbe a des trous).
    let sql = format!(
        "SELECT \
            d::date AS day, \
            COALESCE(SUM(CASE WHEN a.event_type = 'mod_warn' THEN 1 ELSE 0 END), 0) AS warns, \
            COALESCE(SUM(CASE WHEN a.event_type IN ('mod_mute_temp','mod_mute_permanent','mod_mute') THEN 1 ELSE 0 END), 0) AS mutes, \
            COALESCE(SUM(CASE WHEN a.event_type IN ('mod_ban_temp','mod_ban_permanent','mod_ban') THEN 1 ELSE 0 END), 0) AS bans, \
            COALESCE(SUM(CASE WHEN a.event_type = 'mod_kick' THEN 1 ELSE 0 END), 0) AS kicks \
         FROM generate_series( \
                 (CURRENT_DATE - INTERVAL '{days} days')::date, \
                 CURRENT_DATE, \
                 INTERVAL '1 day' \
              ) AS d \
         LEFT JOIN audit_logs a \
             ON a.guild_id = $1 \
             AND a.event_type LIKE 'mod_%' \
             AND a.event_type NOT IN ('mod_unban','mod_unmute') \
             AND a.actor_id IS NOT NULL \
             AND a.created_at::date = d::date \
         GROUP BY d \
         ORDER BY d ASC"
    );

    let rows: Vec<TrendRow> = sqlx::query_as::<_, TrendRow>(&sql)
        .bind(&guild_id)
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError(sentinel_core::domain::errors::DomainError::Internal(format!("modstats trend query: {e}"))))?;

    let dtos = rows
        .into_iter()
        .map(|r| ModstatsTrendDayDto {
            day: r.day.to_string(),
            warns: r.warns,
            mutes: r.mutes,
            bans: r.bans,
            kicks: r.kicks,
        })
        .collect();

    Ok(Json(dtos))
}

#[derive(serde::Deserialize)]
pub struct TrendQuery {
    pub days: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct ModstatsTrendDayDto {
    pub day: String,
    pub warns: i64,
    pub mutes: i64,
    pub bans: i64,
    pub kicks: i64,
}

/// DELETE /api/moderation/actions/{id} — annule une action.
///
/// Comportement selon le type d'action :
/// - `ban*`  : appelle Discord API pour **unban** l'utilisateur puis supprime la ligne.
/// - `mute*` / `timeout` : appelle Discord API pour **retirer le timeout**
///   (`communication_disabled_until = null`) puis supprime la ligne.
/// - `warn` / autre : supprime juste la ligne (pas d'effet Discord natif).
pub async fn delete_action(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let uuid = uuid::Uuid::parse_str(&id)
        .map_err(|_| ApiError(sentinel_core::domain::errors::DomainError::ValidationError("ID invalide".into())))?;

    // Phase 4 : lookup dans `audit_logs` (event_type='mod_*') et plus
    // dans `moderation_actions` qui n'est plus alimentee. action_id est
    // stocke dans `details->>'action_id'`. action_type derive de
    // event_type en strippant le prefixe 'mod_'.
    let row: Option<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT guild_id, target_id, target_name, event_type \
         FROM audit_logs \
         WHERE event_type LIKE 'mod_%' AND details->>'action_id' = $1 \
         LIMIT 1",
    )
    .bind(uuid.to_string())
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError(DomainError::Internal(format!("fetch action: {e}"))))?;

    let Some((guild_id, target_id_opt, target_name_opt, event_type)) = row else {
        return Err(ApiError(sentinel_core::domain::errors::DomainError::NotFound("Action introuvable".into())));
    };
    let target_id = target_id_opt.unwrap_or_default();
    let target_name = target_name_opt.unwrap_or_default();
    let action_type = event_type.strip_prefix("mod_").unwrap_or(&event_type).to_string();

    // Gate RBAC : moderator+ sur la guild concernee.
    check_role_for_guild(
        &state,
        &rbac,
        &guild_id,
        Role::Moderator,
        "moderator+ requis pour annuler une action",
    )
    .await?;

    // Reversal Discord : selon le type d'action, on effectue l'action
    // inverse AVANT de supprimer la ligne. Best-effort : une erreur Discord
    // ne bloque pas la suppression en DB (on log et on continue pour que
    // l'UI reste coherente).
    let lower = action_type.to_lowercase();
    if lower.starts_with("ban") {
        match state.discord_api.unban_user(&guild_id, &target_id).await {
            Ok(()) => tracing::info!(
                guild_id = %guild_id,
                target_id = %target_id,
                target_name = %target_name,
                "Unban Discord applique lors de l'annulation d'une action ban"
            ),
            Err(e) => tracing::warn!(
                error = %e,
                guild_id = %guild_id,
                target_id = %target_id,
                "Echec unban Discord lors de l'annulation — suppression DB quand meme"
            ),
        }
    } else if lower.starts_with("mute") || lower == "timeout" {
        match state.discord_api.remove_timeout(&guild_id, &target_id).await {
            Ok(()) => tracing::info!(
                guild_id = %guild_id,
                target_id = %target_id,
                target_name = %target_name,
                "Timeout Discord retire lors de l'annulation d'une action mute/timeout"
            ),
            Err(e) => tracing::warn!(
                error = %e,
                guild_id = %guild_id,
                target_id = %target_id,
                "Echec remove_timeout Discord lors de l'annulation — suppression DB quand meme"
            ),
        }
    }

    let deleted = state.moderation_uc.delete_action(uuid).await?;
    if deleted {
        // Restitue les points de conduite associes a l'action annulee
        // (warn=1, mute=2, ban=5 selon config guild). Best-effort : si
        // la restitution echoue, on log mais on retourne quand meme 204
        // (l'action est bien supprimee, c'est l'essentiel).
        if let Err(e) = state
            .conduct_uc
            .restore_for_action(&guild_id, &target_id, &action_type)
            .await
        {
            tracing::warn!(
                error = %e,
                guild_id = %guild_id,
                target_id = %target_id,
                action_type = %action_type,
                "Echec restitution points de conduite apres annulation action"
            );
        }
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        Err(ApiError(sentinel_core::domain::errors::DomainError::NotFound("Action introuvable".into())))
    }
}

#[cfg(test)]
#[path = "tests/actions.rs"]
mod tests;

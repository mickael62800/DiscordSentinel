//! Handlers combats : cycle de vie complet (création, transitions, résolution,
//! expiration, annulation) + lectures associées. Délèguent à `state.coude_combats_uc`.

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use super::dto::CombatDto;
use super::dto::CombatQueryParams;
use super::dto::CreateCombatDto;
use super::dto::DefenderSpecialDto;
use super::dto::FullCombatDto;
use super::dto::ResolveCombatDto;
use super::dto::SetBettingDto;
use super::parse_combat_id;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::domain::enums::system::role::Role;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::entities::coude::combat::CombatResolution;
use crate::domain::entities::coude::combat::NewCoudeCombat;
// ── Lecture ──

/// GET /api/coude/{guild_id}/combats — liste des combats
pub async fn list_combats(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<CombatQueryParams>,
) -> Result<Json<Vec<CombatDto>>, ApiError> {
    let limit = params.limit.unwrap_or(crate::domain::entities::coude::limits::DEFAULT_COUDE_COMBATS_LIMIT);
    let combats = state
        .coude_combats_uc
        .list(&guild_id, params.status.as_deref(), limit)
        .await?;
    Ok(Json(combats.iter().map(CombatDto::from).collect()))
}

/// GET /api/coude/combats/{combat_id}
pub async fn get_combat(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
) -> Result<Json<FullCombatDto>, ApiError> {
    let id = parse_combat_id(&combat_id)?;
    let combat = state.coude_combats_uc.get(id).await?;
    Ok(Json(combat.into()))
}

/// GET /api/coude/{guild_id}/combats/pending/attacker/{user_id}
pub async fn get_pending_for_attacker(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<FullCombatDto>>, ApiError> {
    let combat = state
        .coude_combats_uc
        .get_pending_for_attacker(&guild_id, &user_id)
        .await?;
    Ok(Json(combat.map(FullCombatDto::from)))
}

/// GET /api/coude/{guild_id}/combats/pending/defender/{user_id}
pub async fn get_pending_for_defender(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<FullCombatDto>>, ApiError> {
    let combat = state
        .coude_combats_uc
        .get_pending_for_defender(&guild_id, &user_id)
        .await?;
    Ok(Json(combat.map(FullCombatDto::from)))
}

/// GET /api/coude/combats/expired
pub async fn get_expired_combats(
    State(state): State<AppState>,
) -> Result<Json<Vec<FullCombatDto>>, ApiError> {
    let combats = state.coude_combats_uc.list_expired_pending().await?;
    Ok(Json(combats.into_iter().map(FullCombatDto::from).collect()))
}

// ── Cycle de vie ──

/// POST /api/coude/{guild_id}/combats
pub async fn create_combat(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreateCombatDto>,
) -> Result<Json<FullCombatDto>, ApiError> {
    check_role_for_guild(&state, &rbac, &guild_id, Role::Moderator, "moderator+ requis pour create_combat").await?;
    let combat = state
        .coude_combats_uc
        .create(NewCoudeCombat {
            guild_id: guild_id.into(),
            channel_id: dto.channel_id,
            attacker_id: dto.attacker_id,
            attacker_name: dto.attacker_name,
            defender_id: dto.defender_id,
            defender_name: dto.defender_name,
            mise: dto.mise,
            special_attack: dto.special_attack,
        })
        .await?;
    Ok(Json(combat.into()))
}

/// DELETE /api/coude/combats/{combat_id} — annuler un combat pending
/// (effet de bord : marque les paris non résolus comme perdus).
pub async fn cancel_combat(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(combat_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let id = parse_combat_id(&combat_id)?;

    // Phase 7 B — Gate RBAC : moderator+ requis pour annuler un combat coude.
    // Resolution du guild_id via l'UC (ressource-id-based). Si le combat
    // n'existe pas, on laisse `cancel()` remonter le 404 — ne pas masquer.
    if rbac.is_some() {
        if let Some(guild_id) = state.coude_combats_uc.get_guild_id(id).await? {
            check_role_for_guild(
                &state,
                &rbac,
                &guild_id,
                Role::Moderator,
                "moderator+ requis pour annuler un combat",
            )
            .await?;
        }
    }

    state.coude_combats_uc.cancel(id).await?;

    // Sync bilateral : edit l'embed du defi Discord (gris + retire boutons).
    state.broadcaster.broadcast(
        "coude_combat_cancelled",
        serde_json::json!({
            "combat_id": &combat_id,
            "action_id": &combat_id,
            "actor": { "source": "web" },
        }),
    );

    Ok(ok_response())
}

/// POST /api/coude/combats/{combat_id}/resolve
pub async fn resolve_combat(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(combat_id): Path<String>,
    Json(dto): Json<ResolveCombatDto>,
) -> Result<StatusCode, ApiError> {
    let id = parse_combat_id(&combat_id)?;
    if let Some(gid) = state.coude_combats_uc.get_guild_id(id).await? {
        check_role_for_guild(&state, &rbac, &gid, Role::Moderator, "moderator+ requis pour resolve_combat").await?;
    }
    state
        .coude_combats_uc
        .resolve(
            id,
            CombatResolution {
                status: dto.status,
                winner_id: dto.winner_id,
                attacker_roll: dto.attacker_roll,
                defender_roll: dto.defender_roll,
                chaos_event: dto.chaos_event,
                result_message: dto.result_message,
                coins_transferred: dto.coins_transferred.unwrap_or(0),
            },
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/combats/{combat_id}/betting
pub async fn set_combat_betting(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(combat_id): Path<String>,
    Json(dto): Json<SetBettingDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let id = parse_combat_id(&combat_id)?;
    if let Some(gid) = state.coude_combats_uc.get_guild_id(id).await? {
        check_role_for_guild(&state, &rbac, &gid, Role::Moderator, "moderator+ requis pour set_combat_betting").await?;
    }
    let success = state
        .coude_combats_uc
        .set_betting(id, &dto.message_id)
        .await?;
    Ok(Json(serde_json::json!({ "success": success })))
}

/// POST /api/coude/combats/{combat_id}/expire
pub async fn expire_combat(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(combat_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = parse_combat_id(&combat_id)?;
    if let Some(gid) = state.coude_combats_uc.get_guild_id(id).await? {
        check_role_for_guild(&state, &rbac, &gid, Role::Moderator, "moderator+ requis pour expire_combat").await?;
    }
    state.coude_combats_uc.expire(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/coude/{guild_id}/purge
/// Vide totalement toutes les tables Coup de Coude pour une guild donnee.
/// Double-check cote frontend obligatoire.
pub async fn purge_all(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(guild_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    check_role_for_guild(
        &state,
        &rbac,
        &guild_id,
        Role::Owner,
        "owner uniquement pour purger les donnees coude",
    )
    .await?;

    // Le repo possede l'atomicite (tx + ordre des DELETE selon
    // COUDE_PURGE_TABLES). Handler convertit juste le resultat en JSON.
    let counts = state.coude_combats_uc.purge_guild_subsystem(&guild_id).await?;
    let totals: serde_json::Map<String, serde_json::Value> = counts
        .into_iter()
        .map(|(table, rows)| (table, serde_json::Value::from(rows)))
        .collect();

    Ok(Json(serde_json::Value::Object(totals)))
}

/// POST /api/coude/combats/{combat_id}/defender-special
pub async fn set_defender_special(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
    Json(dto): Json<DefenderSpecialDto>,
) -> Result<StatusCode, ApiError> {
    let id = parse_combat_id(&combat_id)?;
    state
        .coude_combats_uc
        .set_defender_special(id, &dto.item_key)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

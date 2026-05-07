//! Handlers paris : place/list/resolve/refund. Le handler `get_betting_combat`
//! est ici parce que c'est une lookup combat utilisée par le flow paris.

use axum::extract::Path;
use axum::extract::State;
use axum::Extension;
use axum::Json;

use super::dto::BetDto;
use super::dto::FullCombatDto;
use super::dto::PlaceBetDto;
use super::dto::PlaceBetResponse;
use super::dto::ResolveBetsDto;
use super::dto::ResolveBetsResponse;
use super::taunts::TauntEventDto;
use super::parse_combat_id;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::entities::coude::bet::NewCoudeBet;

/// POST /api/coude/{guild_id}/bets
///
/// Migration #7 : retourne `PlaceBetResponse { taunt_events }` plutot que
/// 204 No Content. Les taunts eventuels (faillite parieur) sont propages
/// au bot via ce payload (meme pattern que `/donner`).
pub async fn place_bet(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(guild_id): Path<String>,
    Json(dto): Json<PlaceBetDto>,
) -> Result<Json<PlaceBetResponse>, ApiError> {
    check_role_for_guild(&state, &rbac, &guild_id, Role::Moderator, "moderator+ requis pour place_bet").await?;
    let combat_id = parse_combat_id(&dto.combat_id)?;
    let outcome = state
        .coude_bets_uc
        .place(NewCoudeBet {
            guild_id: guild_id.into(),
            combat_id,
            bettor_id: dto.bettor_id,
            bettor_name: dto.bettor_name,
            backed_id: dto.backed_id,
            amount: dto.amount,
        })
        .await?;
    Ok(Json(PlaceBetResponse {
        taunt_events: outcome
            .taunt_events
            .into_iter()
            .map(TauntEventDto::from)
            .collect(),
    }))
}

/// GET /api/coude/combats/{combat_id}/bets
pub async fn get_combat_bets(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
) -> Result<Json<Vec<BetDto>>, ApiError> {
    let id = parse_combat_id(&combat_id)?;
    let bets = state.coude_bets_uc.list_for_combat(id).await?;
    Ok(Json(bets.iter().map(BetDto::from).collect()))
}

/// GET /api/coude/{guild_id}/combats/betting/{user_id}
///
/// Retourne le combat en phase de paris auquel `user_id` participe.
/// Utilise `coude_combats_uc` car c'est une lookup combat, mais le handler
/// vit dans le module bets puisqu'il sert le flow paris.
pub async fn get_betting_combat(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<FullCombatDto>>, ApiError> {
    let combat = state
        .coude_combats_uc
        .get_betting_for_participant(&guild_id, &user_id)
        .await?;
    Ok(Json(combat.map(FullCombatDto::from)))
}

/// POST /api/coude/combats/{combat_id}/resolve-bets
///
/// Résolution pari-mutuel 15% de commission (10% vainqueur / 5% perdant, 85% aux parieurs).
pub async fn resolve_bets(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(combat_id): Path<String>,
    Json(dto): Json<ResolveBetsDto>,
) -> Result<Json<ResolveBetsResponse>, ApiError> {
    let id = parse_combat_id(&combat_id)?;
    // Gate Moderator+: lookup guild_id du combat puis check role.
    if let Some(gid) = state.coude_combats_uc.get_guild_id(id).await? {
        check_role_for_guild(&state, &rbac, &gid, Role::Moderator, "moderator+ requis pour resolve_bets").await?;
    }
    let outcome = state.coude_bets_uc.resolve(id, dto.winner_id).await?;
    let mut resp: ResolveBetsResponse = outcome.plan.into();
    resp.taunt_events = outcome
        .taunt_events
        .into_iter()
        .map(TauntEventDto::from)
        .collect();
    Ok(Json(resp))
}

/// POST /api/coude/combats/{combat_id}/refund-bets
pub async fn refund_bets(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path(combat_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let id = parse_combat_id(&combat_id)?;
    if let Some(gid) = state.coude_combats_uc.get_guild_id(id).await? {
        check_role_for_guild(&state, &rbac, &gid, Role::Moderator, "moderator+ requis pour refund_bets").await?;
    }
    let summary = state.coude_bets_uc.refund(id).await?;
    Ok(Json(serde_json::json!({
        "refunded_count": summary.refunded_count,
        "refunded_total": summary.refunded_total
    })))
}

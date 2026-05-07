//! Handlers joueurs : CRUD, progression (XP/level/stats), stats de combat,
//! coins et HP. Tous délèguent à `state.coude_players_uc`.

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;

use super::dto::AddXpDto;
use super::dto::AddXpResponse;
use super::dto::AdjustCoinsDto;
use super::dto::AmountDto;
use super::dto::FullPlayerDto;
use super::dto::GetOrCreatePlayerDto;
use super::dto::PlayerDto;
use super::dto::RandomPlayersQuery;
use super::dto::RecordDrawDto;
use super::dto::RecordLossDto;
use super::dto::RecordWinDto;
use super::dto::ResetStatsDto;
use super::dto::SpendStatDto;
use super::dto::UpdateClassDto;
use super::dto::UpdateHpDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::rbac::check_role_for_guild;
use crate::adapters::inbound::http::middleware::rbac::RoleContext;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::enums::system::role::Role;
use sentinel_core::domain::entities::coude::player::CombatStat;
use sentinel_core::domain::errors::DomainError;

/// Helper local : gate Moderator+ pour les mutations coude. Pass-through si
/// rbac absent (= appel bot interne sans X-Discord-Token).
async fn gate(
    state: &AppState,
    rbac: &Option<Extension<RoleContext>>,
    guild_id: &str,
    label: &'static str,
) -> Result<(), ApiError> {
    check_role_for_guild(state, rbac, guild_id, Role::Moderator, label).await
}

// ── Listing & utilitaires ──

/// GET /api/coude/{guild_id}/players — liste des joueurs
pub async fn list_players(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<PlayerDto>>, ApiError> {
    let players = state.coude_players_uc.list(&guild_id).await?;
    Ok(Json(players.iter().map(PlayerDto::from).collect()))
}

/// GET /api/coude/guilds — liste distincte des guild_id ayant au moins un joueur
pub async fn get_all_guild_ids(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, ApiError> {
    let guild_ids = state.coude_players_uc.list_guild_ids().await?;
    Ok(Json(guild_ids))
}

/// GET /api/coude/{guild_id}/players/random?count=2
pub async fn get_random_players(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<RandomPlayersQuery>,
) -> Result<Json<Vec<FullPlayerDto>>, ApiError> {
    let count = params.count.unwrap_or(sentinel_core::domain::entities::coude::limits::DEFAULT_COUDE_OPPONENT_COUNT);
    let players = state
        .coude_players_uc
        .random_active(&guild_id, count)
        .await?;
    Ok(Json(players.into_iter().map(FullPlayerDto::from).collect()))
}

// ── CRUD ──

/// POST /api/coude/{guild_id}/players/get-or-create
pub async fn get_or_create_player(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<GetOrCreatePlayerDto>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    let player = state
        .coude_players_uc
        .get_or_create(guild_id.into(), dto.user_id, dto.username)
        .await?;
    Ok(Json(player.into()))
}

/// GET /api/coude/{guild_id}/players/{user_id}
pub async fn get_player(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    let player = state.coude_players_uc.get(&guild_id, &user_id).await?;
    Ok(Json(player.into()))
}

/// PATCH /api/coude/{guild_id}/players/{user_id}/class
pub async fn update_player_class(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UpdateClassDto>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour update_player_class").await?;
    state
        .coude_players_uc
        .update_class(&guild_id, &user_id, &dto.class)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Progression ──

/// POST /api/coude/{guild_id}/players/{user_id}/xp
pub async fn add_xp(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AddXpDto>,
) -> Result<Json<AddXpResponse>, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour add_xp").await?;
    let progress = state
        .coude_players_uc
        .add_xp(&guild_id, &user_id, dto.amount)
        .await?;
    Ok(Json(progress.into()))
}

/// POST /api/coude/{guild_id}/players/{user_id}/spend-stat
pub async fn spend_stat_point(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<SpendStatDto>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour spend_stat_point").await?;
    let stat = CombatStat::parse(&dto.stat).ok_or_else(|| {
        ApiError::from(DomainError::ValidationError(
            "Stat invalide, doit etre 'atk' ou 'def'".into(),
        ))
    })?;
    let player = state
        .coude_players_uc
        .spend_stat_point(&guild_id, &user_id, stat)
        .await?;
    Ok(Json(player.into()))
}

/// POST /api/coude/{guild_id}/players/{user_id}/reset-stats
///
/// Reset atomique : remet ATK/DEF à 0, restitue les points dans `stat_points`
/// et déduit le coût (en coins).
pub async fn reset_stats(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<ResetStatsDto>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    crate::adapters::inbound::http::middleware::component_gates::check_component_role(
        &state, &rbac, &guild_id, "db.reset.coude_stats",
        "role insuffisant pour reset_stats",
    ).await?;
    let player = state
        .coude_players_uc
        .reset_stats(&guild_id, &user_id, dto.cost)
        .await?;
    Ok(Json(player.into()))
}

// ── Stats recording ──

/// POST /api/coude/{guild_id}/players/{user_id}/record-win
pub async fn record_win(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<RecordWinDto>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour record_win").await?;
    state
        .coude_players_uc
        .record_win(&guild_id, &user_id, dto.earned, dto.stolen)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/record-loss
pub async fn record_loss(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<RecordLossDto>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour record_loss").await?;
    state
        .coude_players_uc
        .record_loss(&guild_id, &user_id, dto.lost)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/record-draw
pub async fn record_draw(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<RecordDrawDto>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour record_draw").await?;
    state
        .coude_players_uc
        .record_draw(&guild_id, &user_id, dto.lost)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/increment-cowardice
pub async fn increment_cowardice(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour increment_cowardice").await?;
    let count = state
        .coude_players_uc
        .increment_cowardice(&guild_id, &user_id)
        .await?;
    Ok(Json(serde_json::json!({ "cowardice_count": count })))
}

/// POST /api/coude/{guild_id}/players/{user_id}/increment-chaos
pub async fn increment_chaos(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour increment_chaos").await?;
    state
        .coude_players_uc
        .increment_chaos(&guild_id, &user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Coins ──

/// PATCH /api/coude/players/{guild_id}/{user_id}/coins — ajouter ou retirer des coins.
///
/// Migration wallet finale : delegue directement a `wallet_uc.credit/debit`
/// (ajustement admin). Pas d'update stats `total_earned`/`total_lost` —
/// un ajustement manuel n'est ni un gain ni une perte de gameplay.
pub async fn adjust_coins(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AdjustCoinsDto>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour adjust_coins").await?;
    let delta = dto.amount;
    if delta == 0 {
        return Ok(StatusCode::NO_CONTENT);
    }
    if delta > 0 {
        state
            .wallet_uc
            .credit(&guild_id, &user_id, delta, "coude_adjust", "Ajustement manuel")
            .await?;
    } else {
        state
            .wallet_uc
            .debit(&guild_id, &user_id, -delta, "coude_adjust", "Ajustement manuel")
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/coins-earned
///
/// Migration wallet finale : `wallet_uc.credit` (avec detection auto
/// jackpot) + update stats `total_earned`. Les taunts eventuels ne sont
/// pas propages ici (endpoint fire-and-forget) : ils seraient perdus. Pour
/// les flux qui produisent des gros jackpots (primes combat), preferer un
/// call site plus direct comme `resolve_combat_now_service`.
pub async fn record_coins_earned(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AmountDto>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour record_coins_earned").await?;
    if dto.amount <= 0 {
        return Err(ApiError::from(DomainError::ValidationError(
            "Le montant doit etre positif".into(),
        )));
    }
    state
        .wallet_uc
        .credit(&guild_id, &user_id, dto.amount, "coude_earn", "Gain coude")
        .await?;
    state
        .coude_players_uc
        .record_coins_earned(&guild_id, &user_id, dto.amount)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/coins-lost
///
/// Migration wallet finale : clamp au solde reel (comportement legacy),
/// delegue a `wallet_uc.debit` (detection auto faillite) + update stats
/// `total_lost`. Les taunts de faillite eventuels ne sont pas propages
/// ici (endpoint fire-and-forget).
pub async fn record_coins_lost(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AmountDto>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour record_coins_lost").await?;
    if dto.amount <= 0 {
        return Err(ApiError::from(DomainError::ValidationError(
            "Le montant doit etre positif".into(),
        )));
    }
    // Clamp au solde reel pour preserver le comportement legacy.
    // Regle metier : `domain/entities/wallet.rs::clamp_debit_to_balance`.
    let balance = state.wallet_uc.get_balance(&guild_id, &user_id).await?;
    let actual = sentinel_core::domain::entities::casino::wallet::clamp_debit_to_balance(dto.amount, balance);
    if actual > 0 {
        state
            .wallet_uc
            .debit(&guild_id, &user_id, actual, "coude_loss", "Perte coude")
            .await?;
    }
    // On incremente total_lost du montant reel debite (coherent avec
    // l'ancienne semantique : GREATEST(0, coins - amount) ne comptait que
    // ce qui etait reellement retire).
    state
        .coude_players_uc
        .record_coins_lost(&guild_id, &user_id, actual)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── HP ──

/// POST /api/coude/{guild_id}/players/{user_id}/hp
pub async fn update_hp(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UpdateHpDto>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour update_hp").await?;
    state
        .coude_players_uc
        .update_hp(&guild_id, &user_id, dto.hp_current, dto.hp_max)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/repos — soin complet (full heal)
pub async fn repos(
    State(state): State<AppState>,
    rbac: Option<Extension<RoleContext>>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    gate(&state, &rbac, &guild_id, "moderator+ requis pour repos").await?;
    state.coude_players_uc.full_heal(&guild_id, &user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

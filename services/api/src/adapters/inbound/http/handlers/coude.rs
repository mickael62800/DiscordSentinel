use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

// ── DTOs ──

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CombatDto {
    pub id: String,
    pub guild_id: String,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    pub defender_special: Option<String>,
    pub coins_transferred: Option<i64>,
    pub result_message: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PlayerDto {
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_earned: i64,
    pub total_lost: i64,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub casino_wins: i32,
    pub casino_losses: i32,
    pub level: i32,
    pub xp: i64,
    pub class: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CombatQueryParams {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

// ── New DTOs for expanded endpoints ──

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FullPlayerDto {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub coins: i64,
    pub total_wins: i32,
    pub total_losses: i32,
    pub total_draws: i32,
    pub total_earned: i64,
    pub total_lost: i64,
    pub total_stolen: i64,
    pub cowardice_count: i32,
    pub chaos_events: i32,
    pub casino_wins: i32,
    pub casino_losses: i32,
    pub level: i32,
    pub xp: i64,
    pub stat_points: i32,
    pub atk: i32,
    pub def: i32,
    pub class: Option<String>,
    pub title: Option<String>,
    pub hp_current: i32,
    pub hp_max: i32,
    pub hp_last_regen: Option<String>,
    pub class_changed_at: Option<String>,
    pub repos_last_used: Option<String>,
    pub season: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FullCombatDto {
    pub id: String,
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub special_attack: Option<String>,
    pub defender_special: Option<String>,
    pub coins_transferred: Option<i64>,
    pub result_message: Option<String>,
    pub message_id: Option<String>,
    pub created_at: String,
    pub accepted_at: Option<String>,
    pub resolved_at: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BetDto {
    pub id: String,
    pub combat_id: String,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PrimeDto {
    pub id: String,
    pub guild_id: String,
    pub target_id: String,
    pub target_name: String,
    pub placed_by_id: String,
    pub placed_by_name: String,
    pub amount: i64,
    pub claimed: bool,
    pub claimed_by_id: Option<String>,
    pub claimed_by_name: Option<String>,
    pub claimed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InsuranceDto {
    pub id: String,
    pub is_scam: bool,
    pub expires_at: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LeaderboardEntry {
    pub user_id: String,
    pub username: String,
    pub value: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InventoryItemDto {
    pub guild_id: String,
    pub user_id: String,
    pub item_key: String,
    pub quantity: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EventDto {
    pub id: String,
    pub guild_id: String,
    pub active: bool,
    pub expires_at: String,
    pub created_at: String,
}

// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct GetOrCreatePlayerDto {
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClassDto {
    pub class: String,
}

#[derive(Debug, Deserialize)]
pub struct AddXpDto {
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct AddXpResponse {
    pub new_xp: i64,
    pub new_level: i32,
    pub leveled_up: bool,
    pub stat_points_gained: i32,
}

#[derive(Debug, Deserialize)]
pub struct SpendStatDto {
    pub stat: String,
}

#[derive(Debug, Deserialize)]
pub struct RecordWinDto {
    pub earned: i64,
    pub stolen: i64,
}

#[derive(Debug, Deserialize)]
pub struct RecordLossDto {
    pub lost: i64,
}

#[derive(Debug, Deserialize)]
pub struct RecordDrawDto {
    pub lost: i64,
}

#[derive(Debug, Deserialize)]
pub struct AmountDto {
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct GainDto {
    pub gain: i64,
}

#[derive(Debug, Deserialize)]
pub struct LostDto {
    pub lost: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateCombatDto {
    pub channel_id: Option<String>,
    pub attacker_id: String,
    pub attacker_name: String,
    pub defender_id: String,
    pub defender_name: String,
    pub mise: i64,
    pub special_attack: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveCombatDto {
    pub status: String,
    pub winner_id: Option<String>,
    pub attacker_roll: Option<i32>,
    pub defender_roll: Option<i32>,
    pub chaos_event: Option<String>,
    pub result_message: Option<String>,
    pub coins_transferred: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SetBettingDto {
    pub message_id: String,
}

#[derive(Debug, Deserialize)]
pub struct DefenderSpecialDto {
    pub item_key: String,
}

#[derive(Debug, Deserialize)]
pub struct PlaceBetDto {
    pub combat_id: String,
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct ResolveBetsDto {
    pub winner_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BetResult {
    pub bettor_id: String,
    pub bettor_name: String,
    pub backed_id: String,
    pub amount_bet: i64,
    pub payout: i64,
    pub won: bool,
}

#[derive(Debug, Serialize)]
pub struct FighterBetBonus {
    pub winner_id: String,
    pub winner_bonus: i64,
    pub loser_id: String,
    pub loser_bonus: i64,
}

#[derive(Debug, Serialize)]
pub struct ResolveBetsResponse {
    pub results: Vec<BetResult>,
    pub fighter_bonus: Option<FighterBetBonus>,
}

#[derive(Debug, Deserialize)]
pub struct DurationDto {
    pub duration_secs: i64,
}

#[derive(Debug, Deserialize)]
pub struct TransferCoinsDto {
    pub from_id: String,
    pub to_id: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct StealDto {
    pub thief_id: String,
    pub victim_id: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreatePrimeDto {
    pub target_id: String,
    pub target_name: String,
    pub placed_by_id: String,
    pub placed_by_name: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct ClaimPrimesDto {
    pub target_id: String,
    pub claimer_id: String,
    pub claimer_name: String,
}

#[derive(Debug, Deserialize)]
pub struct BuyInsuranceDto {
    pub user_id: String,
    pub is_scam: bool,
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardQueryParams {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RandomPlayersQuery {
    pub count: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DailyChaosDto {
    pub loser_id: String,
    pub loser_name: String,
    pub winner_id: String,
    pub winner_name: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize)]
pub struct AddItemDto {
    pub item_key: String,
}

#[derive(Debug, Deserialize)]
pub struct UseItemDto {
    pub item_key: String,
}

// ── Handlers (existing) ──

/// GET /api/coude/{guild_id}/combats — liste des combats
pub async fn list_combats(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<CombatQueryParams>,
) -> Result<Json<Vec<CombatDto>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(200);
    let status_filter = params.status.as_deref().unwrap_or("all");

    let combats = if status_filter == "all" {
        sqlx::query_as::<_, CombatDto>(
            r#"SELECT
                c.id::text, c.guild_id, c.attacker_id,
                COALESCE(pa.username, c.attacker_id) as attacker_name,
                c.defender_id,
                COALESCE(pd.username, c.defender_id) as defender_name,
                c.mise, c.status, c.winner_id,
                c.attacker_roll, c.defender_roll,
                c.chaos_event, c.special_attack, c.defender_special,
                c.coins_transferred, c.result_message,
                c.created_at::text, c.resolved_at::text
            FROM coude_combats c
            LEFT JOIN coude_players pa ON pa.guild_id = c.guild_id AND pa.user_id = c.attacker_id
            LEFT JOIN coude_players pd ON pd.guild_id = c.guild_id AND pd.user_id = c.defender_id
            WHERE c.guild_id = $1
            ORDER BY c.created_at DESC
            LIMIT $2"#,
        )
        .bind(&guild_id)
        .bind(limit)
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
    } else {
        sqlx::query_as::<_, CombatDto>(
            r#"SELECT
                c.id::text, c.guild_id, c.attacker_id,
                COALESCE(pa.username, c.attacker_id) as attacker_name,
                c.defender_id,
                COALESCE(pd.username, c.defender_id) as defender_name,
                c.mise, c.status, c.winner_id,
                c.attacker_roll, c.defender_roll,
                c.chaos_event, c.special_attack, c.defender_special,
                c.coins_transferred, c.result_message,
                c.created_at::text, c.resolved_at::text
            FROM coude_combats c
            LEFT JOIN coude_players pa ON pa.guild_id = c.guild_id AND pa.user_id = c.attacker_id
            LEFT JOIN coude_players pd ON pd.guild_id = c.guild_id AND pd.user_id = c.defender_id
            WHERE c.guild_id = $1 AND c.status = $2
            ORDER BY c.created_at DESC
            LIMIT $3"#,
        )
        .bind(&guild_id)
        .bind(status_filter)
        .bind(limit)
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
    };

    Ok(Json(combats))
}

/// GET /api/coude/{guild_id}/players — liste des joueurs
pub async fn list_players(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<PlayerDto>>, ApiError> {
    let players = sqlx::query_as::<_, PlayerDto>(
        r#"SELECT user_id, username, coins,
            total_wins, total_losses, total_draws,
            total_earned, total_lost, total_stolen,
            cowardice_count, casino_wins, casino_losses,
            level, xp, class, title
        FROM coude_players
        WHERE guild_id = $1
        ORDER BY coins DESC
        LIMIT 200"#,
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(players))
}

/// DELETE /api/coude/combats/{combat_id} — annuler un combat pending
pub async fn cancel_combat(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE coude_combats SET status = 'expired', resolved_at = NOW() WHERE id = $1::uuid AND status = 'pending'"
    )
    .bind(&combat_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Combat introuvable ou deja resolu".into()).into());
    }

    // Rembourser les paris si existants
    if let Err(e) = sqlx::query(
        "UPDATE coude_bets SET won = false WHERE combat_id = $1::uuid AND won IS NULL"
    )
    .bind(&combat_id)
    .execute(&state.pg_pool)
    .await {
        tracing::warn!(error = %e, combat_id = %combat_id, "Echec remboursement paris apres annulation combat");
    }

    Ok(ok_response())
}

// ── Adjust coins ──

#[derive(Debug, Deserialize)]
pub struct AdjustCoinsDto {
    pub amount: i64,
}

/// PATCH /api/coude/players/{guild_id}/{user_id}/coins — ajouter ou retirer des coins
pub async fn adjust_coins(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AdjustCoinsDto>,
) -> Result<StatusCode, ApiError> {
    // Empecher le solde de devenir negatif
    let result = sqlx::query(
        "UPDATE coude_players SET coins = GREATEST(0, coins + $1), updated_at = NOW() WHERE guild_id = $2 AND user_id = $3"
    )
    .bind(dto.amount)
    .bind(&guild_id)
    .bind(&user_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

// ════════════════════════════════════════════════════════════════════════
// ── NEW HANDLERS ──
// ════════════════════════════════════════════════════════════════════════

// ── Helper: XP formula & title ──

fn xp_for_level(n: i32) -> i64 {
    let n = n as i64;
    50 * n * n + 50 * n
}

const MAX_LEVEL: i32 = 25;

fn title_for_level(level: i32) -> &'static str {
    match level {
        1..=4 => "Debutant",
        5..=9 => "Bagarreur",
        10..=14 => "Guerrier",
        15..=19 => "Veteran",
        20..=24 => "Champion",
        25 => "Inarretable",
        _ => "Debutant",
    }
}

// ── 1. Player CRUD ──

/// POST /api/coude/{guild_id}/players/get-or-create
pub async fn get_or_create_player(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<GetOrCreatePlayerDto>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    let player = sqlx::query_as::<_, FullPlayerDto>(
        r#"INSERT INTO coude_players (guild_id, user_id, username)
        VALUES ($1, $2, $3)
        ON CONFLICT (guild_id, user_id) DO UPDATE SET username = EXCLUDED.username, updated_at = NOW()
        RETURNING guild_id, user_id, username, coins,
            total_wins, total_losses, total_draws,
            total_earned, total_lost, total_stolen,
            cowardice_count, chaos_events, casino_wins, casino_losses,
            level, xp, stat_points, atk, def, class, title,
            hp_current, hp_max, hp_last_regen::text, class_changed_at::text, repos_last_used::text, season,
            created_at::text, updated_at::text"#,
    )
    .bind(&guild_id)
    .bind(&dto.user_id)
    .bind(&dto.username)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(player))
}

/// GET /api/coude/{guild_id}/players/{user_id}
pub async fn get_player(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    let player = sqlx::query_as::<_, FullPlayerDto>(
        r#"SELECT guild_id, user_id, username, coins,
            total_wins, total_losses, total_draws,
            total_earned, total_lost, total_stolen,
            cowardice_count, chaos_events, casino_wins, casino_losses,
            level, xp, stat_points, atk, def, class, title,
            hp_current, hp_max, hp_last_regen::text, class_changed_at::text, repos_last_used::text, season,
            created_at::text, updated_at::text
        FROM coude_players WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
    .ok_or_else(|| ApiError::from(DomainError::NotFound("Joueur introuvable".into())))?;

    Ok(Json(player))
}

/// PATCH /api/coude/{guild_id}/players/{user_id}/class
pub async fn update_player_class(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UpdateClassDto>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        "UPDATE coude_players SET class = $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(&dto.class)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/xp
pub async fn add_xp(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AddXpDto>,
) -> Result<Json<AddXpResponse>, ApiError> {
    let mut tx = state
        .pg_pool
        .begin()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Lock the player row
    let row = sqlx::query_as::<_, (i64, i32, i32)>(
        "SELECT xp, level, stat_points FROM coude_players WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
    .ok_or_else(|| ApiError::from(DomainError::NotFound("Joueur introuvable".into())))?;

    let (mut current_xp, mut current_level, mut current_stat_points) = row;
    let old_level = current_level;

    current_xp += dto.amount;

    // Process level ups
    while current_level < MAX_LEVEL && current_xp >= xp_for_level(current_level + 1) {
        current_level += 1;
        current_stat_points += 3;
    }

    let leveled_up = current_level > old_level;
    let stat_points_gained = (current_level - old_level) * 3;
    let new_title = title_for_level(current_level);

    sqlx::query(
        "UPDATE coude_players SET xp = $3, level = $4, stat_points = $5, title = $6, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(current_xp)
    .bind(current_level)
    .bind(current_stat_points)
    .bind(new_title)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(AddXpResponse {
        new_xp: current_xp,
        new_level: current_level,
        leveled_up,
        stat_points_gained,
    }))
}

/// POST /api/coude/{guild_id}/players/{user_id}/spend-stat
pub async fn spend_stat_point(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<SpendStatDto>,
) -> Result<Json<FullPlayerDto>, ApiError> {
    // Validate stat name
    let stat_col = match dto.stat.as_str() {
        "atk" => "atk",
        "def" => "def",
        _ => {
            return Err(DomainError::ValidationError(
                "Stat invalide, doit etre 'atk' ou 'def'".into(),
            )
            .into())
        }
    };

    // Use dynamic SQL safely since stat_col is validated above
    let query = format!(
        r#"UPDATE coude_players SET {stat} = {stat} + 1, stat_points = stat_points - 1, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2 AND stat_points >= 1
        RETURNING guild_id, user_id, username, coins,
            total_wins, total_losses, total_draws,
            total_earned, total_lost, total_stolen,
            cowardice_count, chaos_events, casino_wins, casino_losses,
            level, xp, stat_points, atk, def, class, title,
            hp_current, hp_max, hp_last_regen::text, class_changed_at::text, repos_last_used::text, season,
            created_at::text, updated_at::text"#,
        stat = stat_col
    );

    let player = sqlx::query_as::<_, FullPlayerDto>(&query)
        .bind(&guild_id)
        .bind(&user_id)
        .fetch_optional(&state.pg_pool)
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
        .ok_or_else(|| {
            ApiError::from(DomainError::ValidationError(
                "Joueur introuvable ou pas de stat_points disponibles".into(),
            ))
        })?;

    Ok(Json(player))
}

// ── 6-12. Stats recording ──

/// POST /api/coude/{guild_id}/players/{user_id}/record-win
pub async fn record_win(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<RecordWinDto>,
) -> Result<StatusCode, ApiError> {
    if dto.earned < 0 || dto.stolen < 0 {
        return Err(DomainError::ValidationError("Les montants ne peuvent pas etre negatifs".into()).into());
    }
    let result = sqlx::query(
        r#"UPDATE coude_players SET total_wins = total_wins + 1, coins = coins + $3,
        total_earned = total_earned + $3, total_stolen = total_stolen + $4, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(dto.earned)
    .bind(dto.stolen)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/record-loss
pub async fn record_loss(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<RecordLossDto>,
) -> Result<StatusCode, ApiError> {
    // Utiliser LEAST pour ne perdre que ce qu'on a (pas de dette)
    let result = sqlx::query(
        r#"UPDATE coude_players SET total_losses = total_losses + 1,
        coins = coins - LEAST(coins, $3),
        total_lost = total_lost + LEAST(coins, $3), updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(dto.lost)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/record-draw
pub async fn record_draw(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<RecordDrawDto>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        r#"UPDATE coude_players SET total_draws = total_draws + 1, coins = GREATEST(0, coins - $3),
        total_lost = total_lost + $3, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(dto.lost)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/increment-cowardice
pub async fn increment_cowardice(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, (i32,)>(
        r#"UPDATE coude_players SET cowardice_count = cowardice_count + 1, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2
        RETURNING cowardice_count"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
    .ok_or_else(|| ApiError::from(DomainError::NotFound("Joueur introuvable".into())))?;

    Ok(Json(
        serde_json::json!({ "cowardice_count": row.0 }),
    ))
}

/// POST /api/coude/{guild_id}/players/{user_id}/increment-chaos
pub async fn increment_chaos(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        "UPDATE coude_players SET chaos_events = chaos_events + 1, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/coins-earned
pub async fn record_coins_earned(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AmountDto>,
) -> Result<StatusCode, ApiError> {
    if dto.amount <= 0 {
        return Err(DomainError::ValidationError("Le montant doit etre positif".into()).into());
    }
    let result = sqlx::query(
        r#"UPDATE coude_players SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(dto.amount)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/coins-lost
pub async fn record_coins_lost(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AmountDto>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        r#"UPDATE coude_players SET coins = GREATEST(0, coins - $3), total_lost = total_lost + $3, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(dto.amount)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── 13-16. Casino ──

/// POST /api/coude/{guild_id}/players/{user_id}/casino-win
pub async fn record_casino_win(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<GainDto>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        r#"UPDATE coude_players SET casino_wins = casino_wins + 1, coins = coins + $3,
        total_earned = total_earned + $3, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(dto.gain)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    // Log du gain pour le tracking quotidien
    sqlx::query(
        "INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(dto.gain)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/casino-loss
pub async fn record_casino_loss(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<LostDto>,
) -> Result<StatusCode, ApiError> {
    // Log de la perte pour le tracking quotidien (montant negatif)
    sqlx::query(
        "INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(-dto.lost)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let result = sqlx::query(
        r#"UPDATE coude_players SET casino_losses = casino_losses + 1, coins = GREATEST(0, coins - $3),
        total_lost = total_lost + $3, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(dto.lost)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Joueur introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/players/{user_id}/casino-faillite
pub async fn record_casino_faillite(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Lire les coins actuels avant la faillite
    let coins_before = sqlx::query_as::<_, (i64,)>(
        "SELECT coins FROM coude_players WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
    .ok_or_else(|| ApiError::from(DomainError::NotFound("Joueur introuvable".into())))?
    .0;

    // Mettre a 0
    let row = sqlx::query_as::<_, (i64,)>(
        r#"UPDATE coude_players SET casino_losses = casino_losses + 1, total_lost = total_lost + coins,
        coins = 0, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2
        RETURNING total_lost"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Log de la faillite pour le tracking quotidien
    if coins_before > 0 {
        sqlx::query(
            "INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)",
        )
        .bind(&guild_id)
        .bind(&user_id)
        .bind(-coins_before)
        .execute(&state.pg_pool)
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;
    }

    Ok(Json(serde_json::json!({ "total_lost": row.0 })))
}

/// GET /api/coude/{guild_id}/players/{user_id}/casino-today
pub async fn count_casino_today(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64,)>(
        r#"SELECT COUNT(*) FROM coude_cooldowns
        WHERE guild_id = $1 AND user_id = $2 AND action = 'casino' AND expires_at > NOW() - INTERVAL '24 hours'"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(serde_json::json!({ "count": row.0 })))
}

/// GET /api/coude/{guild_id}/players/{user_id}/casino-gains-today
/// Calcule la somme des gains nets au casino dans les dernieres 24h
/// via la table coude_casino_log (gains positifs uniquement)
pub async fn sum_casino_gains_today(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64,)>(
        r#"SELECT COALESCE(SUM(amount), 0)::bigint FROM coude_casino_log
        WHERE guild_id = $1 AND user_id = $2
        AND amount > 0 AND created_at > NOW() - INTERVAL '24 hours'"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(serde_json::json!({ "total": row.0 })))
}

/// GET /api/coude/{guild_id}/players/{user_id}/steal-today
/// Compte le nombre de vols effectues dans les dernieres 24h
pub async fn count_steal_today(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64,)>(
        r#"SELECT COUNT(*) FROM coude_cooldowns
        WHERE guild_id = $1 AND user_id = $2 AND action = 'voler' AND expires_at > NOW() - INTERVAL '24 hours'"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(serde_json::json!({ "count": row.0 })))
}

// ── 17-25. Combat lifecycle ──

/// POST /api/coude/{guild_id}/combats
pub async fn create_combat(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreateCombatDto>,
) -> Result<Json<FullCombatDto>, ApiError> {
    let combat = sqlx::query_as::<_, FullCombatDto>(
        r#"INSERT INTO coude_combats (guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name, mise, special_attack)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id::text, guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name,
            mise, status, winner_id, attacker_roll, defender_roll, chaos_event, special_attack,
            defender_special, coins_transferred, result_message, message_id,
            created_at::text, accepted_at::text, resolved_at::text"#,
    )
    .bind(&guild_id)
    .bind(&dto.channel_id)
    .bind(&dto.attacker_id)
    .bind(&dto.attacker_name)
    .bind(&dto.defender_id)
    .bind(&dto.defender_name)
    .bind(dto.mise)
    .bind(&dto.special_attack)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(combat))
}

/// GET /api/coude/combats/{combat_id}
pub async fn get_combat(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
) -> Result<Json<FullCombatDto>, ApiError> {
    let combat = sqlx::query_as::<_, FullCombatDto>(
        r#"SELECT id::text, guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name,
            mise, status, winner_id, attacker_roll, defender_roll, chaos_event, special_attack,
            defender_special, coins_transferred, result_message, message_id,
            created_at::text, accepted_at::text, resolved_at::text
        FROM coude_combats WHERE id = $1::uuid"#,
    )
    .bind(&combat_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
    .ok_or_else(|| ApiError::from(DomainError::NotFound("Combat introuvable".into())))?;

    Ok(Json(combat))
}

/// GET /api/coude/{guild_id}/combats/pending/attacker/{user_id}
pub async fn get_pending_for_attacker(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<FullCombatDto>>, ApiError> {
    let combat = sqlx::query_as::<_, FullCombatDto>(
        r#"SELECT id::text, guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name,
            mise, status, winner_id, attacker_roll, defender_roll, chaos_event, special_attack,
            defender_special, coins_transferred, result_message, message_id,
            created_at::text, accepted_at::text, resolved_at::text
        FROM coude_combats WHERE guild_id = $1 AND attacker_id = $2 AND status = 'pending'
        ORDER BY created_at DESC LIMIT 1"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(combat))
}

/// GET /api/coude/{guild_id}/combats/pending/defender/{user_id}
pub async fn get_pending_for_defender(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<FullCombatDto>>, ApiError> {
    let combat = sqlx::query_as::<_, FullCombatDto>(
        r#"SELECT id::text, guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name,
            mise, status, winner_id, attacker_roll, defender_roll, chaos_event, special_attack,
            defender_special, coins_transferred, result_message, message_id,
            created_at::text, accepted_at::text, resolved_at::text
        FROM coude_combats WHERE guild_id = $1 AND defender_id = $2 AND status = 'pending'
        ORDER BY created_at DESC LIMIT 1"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(combat))
}

/// POST /api/coude/combats/{combat_id}/resolve
pub async fn resolve_combat(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
    Json(dto): Json<ResolveCombatDto>,
) -> Result<StatusCode, ApiError> {
    // Guard : ne resoudre que les combats en cours (pending, accepted, betting)
    // Empeche la double resolution par race condition bot/worker
    let result = sqlx::query(
        r#"UPDATE coude_combats SET status = $2, winner_id = $3, attacker_roll = $4, defender_roll = $5,
        chaos_event = $6, result_message = $7, coins_transferred = $8, resolved_at = NOW()
        WHERE id = $1::uuid AND status IN ('pending', 'accepted', 'betting')"#,
    )
    .bind(&combat_id)
    .bind(&dto.status)
    .bind(&dto.winner_id)
    .bind(dto.attacker_roll)
    .bind(dto.defender_roll)
    .bind(&dto.chaos_event)
    .bind(&dto.result_message)
    .bind(dto.coins_transferred)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::Conflict("Combat deja resolu ou introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/combats/{combat_id}/betting
pub async fn set_combat_betting(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
    Json(dto): Json<SetBettingDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE coude_combats SET status = 'betting', accepted_at = NOW(), message_id = $1 WHERE id = $2::uuid AND status = 'pending'",
    )
    .bind(&dto.message_id)
    .bind(&combat_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(
        serde_json::json!({ "success": result.rows_affected() > 0 }),
    ))
}

/// POST /api/coude/combats/{combat_id}/expire
pub async fn expire_combat(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        "UPDATE coude_combats SET status = 'expired', resolved_at = NOW() WHERE id = $1::uuid",
    )
    .bind(&combat_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Combat introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/combats/{combat_id}/defender-special
pub async fn set_defender_special(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
    Json(dto): Json<DefenderSpecialDto>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        "UPDATE coude_combats SET defender_special = $1 WHERE id = $2::uuid",
    )
    .bind(&dto.item_key)
    .bind(&combat_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Combat introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/coude/combats/expired
pub async fn get_expired_combats(
    State(state): State<AppState>,
) -> Result<Json<Vec<FullCombatDto>>, ApiError> {
    let combats = sqlx::query_as::<_, FullCombatDto>(
        r#"SELECT id::text, guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name,
            mise, status, winner_id, attacker_roll, defender_roll, chaos_event, special_attack,
            defender_special, coins_transferred, result_message, message_id,
            created_at::text, accepted_at::text, resolved_at::text
        FROM coude_combats WHERE status = 'pending' AND created_at < NOW() - INTERVAL '24 hours'"#,
    )
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(combats))
}

// ── 26-30. Bets ──

/// POST /api/coude/{guild_id}/bets
pub async fn place_bet(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<PlaceBetDto>,
) -> Result<StatusCode, ApiError> {
    // Validation du montant
    if dto.amount <= 0 {
        return Err(DomainError::ValidationError("Le montant du pari doit etre positif".into()).into());
    }

    let mut tx = state
        .pg_pool
        .begin()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Verifier que le combat existe et est en phase de paris
    let combat_status = sqlx::query_as::<_, (String, String, String)>(
        "SELECT status, attacker_id, defender_id FROM coude_combats WHERE id = $1::uuid",
    )
    .bind(&dto.combat_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let (status, attacker_id, defender_id) = combat_status
        .ok_or_else(|| ApiError::from(DomainError::NotFound("Combat introuvable".into())))?;

    if status != "betting" {
        return Err(DomainError::ValidationError("Les paris ne sont pas ouverts pour ce combat".into()).into());
    }

    // Empecher les participants de parier sur leur propre combat
    if dto.bettor_id == attacker_id || dto.bettor_id == defender_id {
        return Err(DomainError::ValidationError("Un participant ne peut pas parier sur son propre combat".into()).into());
    }

    // Verifier le solde du parieur
    let bettor = sqlx::query_as::<_, (i64,)>(
        "SELECT coins FROM coude_players WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(&guild_id)
    .bind(&dto.bettor_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let (bettor_coins,) = bettor
        .ok_or_else(|| ApiError::from(DomainError::NotFound("Parieur introuvable".into())))?;

    if bettor_coins < dto.amount {
        return Err(DomainError::ValidationError(
            format!("Solde insuffisant ({} coins, {} requis)", bettor_coins, dto.amount),
        ).into());
    }

    // Debiter le parieur
    sqlx::query(
        "UPDATE coude_players SET coins = coins - $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&dto.bettor_id)
    .bind(dto.amount)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Enregistrer le pari
    sqlx::query(
        r#"INSERT INTO coude_bets (guild_id, combat_id, bettor_id, bettor_name, backed_id, amount)
        VALUES ($1, $2::uuid, $3, $4, $5, $6)"#,
    )
    .bind(&guild_id)
    .bind(&dto.combat_id)
    .bind(&dto.bettor_id)
    .bind(&dto.bettor_name)
    .bind(&dto.backed_id)
    .bind(dto.amount)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/coude/combats/{combat_id}/bets
pub async fn get_combat_bets(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
) -> Result<Json<Vec<BetDto>>, ApiError> {
    let bets = sqlx::query_as::<_, BetDto>(
        r#"SELECT id::text, combat_id::text, bettor_id, bettor_name, backed_id, amount
        FROM coude_bets WHERE combat_id = $1::uuid"#,
    )
    .bind(&combat_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(bets))
}

/// GET /api/coude/{guild_id}/combats/betting/{user_id}
pub async fn get_betting_combat(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<FullCombatDto>>, ApiError> {
    let combat = sqlx::query_as::<_, FullCombatDto>(
        r#"SELECT id::text, guild_id, channel_id, attacker_id, attacker_name, defender_id, defender_name,
            mise, status, winner_id, attacker_roll, defender_roll, chaos_event, special_attack,
            defender_special, coins_transferred, result_message, message_id,
            created_at::text, accepted_at::text, resolved_at::text
        FROM coude_combats WHERE guild_id = $1 AND (attacker_id = $2 OR defender_id = $2)
        AND status = 'betting' ORDER BY created_at DESC LIMIT 1"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(combat))
}

/// POST /api/coude/combats/{combat_id}/resolve-bets
///
/// Full pari-mutuel bet resolution with 15% commission for fighters.
pub async fn resolve_bets(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
    Json(dto): Json<ResolveBetsDto>,
) -> Result<Json<ResolveBetsResponse>, ApiError> {
    let mut tx = state
        .pg_pool
        .begin()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Get combat info to know fighters and guild
    let combat = sqlx::query_as::<_, (String, String, String)>(
        "SELECT guild_id, attacker_id, defender_id FROM coude_combats WHERE id = $1::uuid",
    )
    .bind(&combat_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
    .ok_or_else(|| ApiError::from(DomainError::NotFound("Combat introuvable".into())))?;

    let (guild_id, attacker_id, defender_id) = combat;

    // Get all bets
    #[derive(Debug, sqlx::FromRow)]
    struct BetRow {
        id: i64,
        bettor_id: String,
        bettor_name: String,
        backed_id: String,
        amount: i64,
    }

    let bets = sqlx::query_as::<_, BetRow>(
        "SELECT id, bettor_id, bettor_name, backed_id, amount FROM coude_bets WHERE combat_id = $1::uuid",
    )
    .bind(&combat_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if bets.is_empty() {
        tx.commit()
            .await
            .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;
        return Ok(Json(ResolveBetsResponse {
            results: vec![],
            fighter_bonus: None,
        }));
    }

    let total_pot: i64 = bets.iter().map(|b| b.amount).sum();
    let mut results = Vec::new();

    match &dto.winner_id {
        Some(winner_id) => {
            // Commission: 15% total (10% for winner fighter, 5% for loser fighter)
            let commission = (total_pot as f64 * 0.15).round() as i64;
            let winner_bonus = (total_pot as f64 * 0.10).round() as i64;
            let loser_bonus = commission - winner_bonus;
            let distributable = total_pot - commission;

            let loser_id = if *winner_id == attacker_id {
                &defender_id
            } else {
                &attacker_id
            };

            // Calculate winner and loser pools
            let winner_pool: i64 = bets
                .iter()
                .filter(|b| b.backed_id == *winner_id)
                .map(|b| b.amount)
                .sum();
            let _loser_pool: i64 = total_pot - winner_pool;

            // Distribute to bettors
            for bet in &bets {
                if bet.backed_id == *winner_id {
                    // Winning bettor gets proportional share
                    let share = if winner_pool > 0 {
                        ((bet.amount as f64 / winner_pool as f64) * distributable as f64).round()
                            as i64
                    } else {
                        0
                    };
                    let payout = share;

                    // Credit bettor
                    sqlx::query(
                        "UPDATE coude_players SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
                    )
                    .bind(&guild_id)
                    .bind(&bet.bettor_id)
                    .bind(payout)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

                    // Mark bet as won
                    sqlx::query("UPDATE coude_bets SET won = true, payout = $2 WHERE id = $1")
                        .bind(bet.id)
                        .bind(payout)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

                    results.push(BetResult {
                        bettor_id: bet.bettor_id.clone(),
                        bettor_name: bet.bettor_name.clone(),
                        backed_id: bet.backed_id.clone(),
                        amount_bet: bet.amount,
                        payout,
                        won: true,
                    });
                } else {
                    // Losing bettor
                    sqlx::query("UPDATE coude_bets SET won = false, payout = 0 WHERE id = $1")
                        .bind(bet.id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

                    results.push(BetResult {
                        bettor_id: bet.bettor_id.clone(),
                        bettor_name: bet.bettor_name.clone(),
                        backed_id: bet.backed_id.clone(),
                        amount_bet: bet.amount,
                        payout: 0,
                        won: false,
                    });
                }
            }

            // Pay fighter bonuses
            if winner_bonus > 0 {
                sqlx::query(
                    "UPDATE coude_players SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
                )
                .bind(&guild_id)
                .bind(winner_id)
                .bind(winner_bonus)
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;
            }

            if loser_bonus > 0 {
                sqlx::query(
                    "UPDATE coude_players SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
                )
                .bind(&guild_id)
                .bind(loser_id)
                .bind(loser_bonus)
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;
            }

            tx.commit()
                .await
                .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

            Ok(Json(ResolveBetsResponse {
                results,
                fighter_bonus: Some(FighterBetBonus {
                    winner_id: winner_id.clone(),
                    winner_bonus,
                    loser_id: loser_id.clone(),
                    loser_bonus,
                }),
            }))
        }
        None => {
            // Draw or no winner: refund everyone
            for bet in &bets {
                sqlx::query(
                    "UPDATE coude_players SET coins = coins + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
                )
                .bind(&guild_id)
                .bind(&bet.bettor_id)
                .bind(bet.amount)
                .execute(&mut *tx)
                .await
                .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

                sqlx::query("UPDATE coude_bets SET won = false, payout = $2 WHERE id = $1")
                    .bind(bet.id)
                    .bind(bet.amount)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

                results.push(BetResult {
                    bettor_id: bet.bettor_id.clone(),
                    bettor_name: bet.bettor_name.clone(),
                    backed_id: bet.backed_id.clone(),
                    amount_bet: bet.amount,
                    payout: bet.amount,
                    won: false,
                });
            }

            tx.commit()
                .await
                .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

            Ok(Json(ResolveBetsResponse {
                results,
                fighter_bonus: None,
            }))
        }
    }
}

/// POST /api/coude/combats/{combat_id}/refund-bets
pub async fn refund_bets(
    State(state): State<AppState>,
    Path(combat_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut tx = state
        .pg_pool
        .begin()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Get combat guild_id for player updates
    let guild_id = sqlx::query_as::<_, (String,)>(
        "SELECT guild_id FROM coude_combats WHERE id = $1::uuid",
    )
    .bind(&combat_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?
    .ok_or_else(|| ApiError::from(DomainError::NotFound("Combat introuvable".into())))?
    .0;

    #[derive(Debug, sqlx::FromRow)]
    struct RefundBetRow {
        id: i64,
        bettor_id: String,
        amount: i64,
    }

    let bets = sqlx::query_as::<_, RefundBetRow>(
        "SELECT id, bettor_id, amount FROM coude_bets WHERE combat_id = $1::uuid AND won IS NULL",
    )
    .bind(&combat_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let mut refunded = 0i64;
    for bet in &bets {
        sqlx::query(
            "UPDATE coude_players SET coins = coins + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&guild_id)
        .bind(&bet.bettor_id)
        .bind(bet.amount)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

        sqlx::query("UPDATE coude_bets SET won = false, payout = $2 WHERE id = $1")
            .bind(bet.id)
            .bind(bet.amount)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

        refunded += bet.amount;
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(serde_json::json!({
        "refunded_count": bets.len(),
        "refunded_total": refunded
    })))
}

// ── 31-32. Cooldowns ──

/// GET /api/coude/{guild_id}/cooldown/{user_id}/{action}
pub async fn check_cooldown(
    State(state): State<AppState>,
    Path((guild_id, user_id, action)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, (String,)>(
        r#"SELECT expires_at::text FROM coude_cooldowns
        WHERE guild_id = $1 AND user_id = $2 AND action = $3 AND expires_at > NOW()"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(&action)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let expires_at = row.map(|r| r.0);
    Ok(Json(serde_json::json!({ "expires_at": expires_at })))
}

/// POST /api/coude/{guild_id}/cooldown/{user_id}/{action}
pub async fn set_cooldown(
    State(state): State<AppState>,
    Path((guild_id, user_id, action)): Path<(String, String, String)>,
    Json(dto): Json<DurationDto>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        r#"INSERT INTO coude_cooldowns (guild_id, user_id, action, expires_at)
        VALUES ($1, $2, $3, NOW() + make_interval(secs => $4::double precision))
        ON CONFLICT (guild_id, user_id, action)
        DO UPDATE SET expires_at = NOW() + make_interval(secs => $4::double precision)"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(&action)
    .bind(dto.duration_secs as f64)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

// ── 33-34. Economy ──

/// POST /api/coude/{guild_id}/transfer
pub async fn transfer_coins(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<TransferCoinsDto>,
) -> Result<StatusCode, ApiError> {
    if dto.amount <= 0 {
        return Err(DomainError::ValidationError("Le montant doit etre positif".into()).into());
    }

    let mut tx = state
        .pg_pool
        .begin()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Lock + verifier le solde de l'expediteur
    let sender = sqlx::query_as::<_, (i64,)>(
        "SELECT coins FROM coude_players WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(&guild_id)
    .bind(&dto.from_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let (sender_coins,) = sender.ok_or_else(|| ApiError::from(DomainError::NotFound("Expediteur introuvable".into())))?;

    if sender_coins < dto.amount {
        return Err(DomainError::ValidationError(
            format!("Solde insuffisant ({} coins, {} requis)", sender_coins, dto.amount),
        ).into());
    }

    // Debit sender (solde verifie)
    sqlx::query(
        "UPDATE coude_players SET coins = coins - $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&dto.from_id)
    .bind(dto.amount)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Credit receiver
    let result = sqlx::query(
        "UPDATE coude_players SET coins = coins + $3, updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&dto.to_id)
    .bind(dto.amount)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Destinataire introuvable".into()).into());
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/steal
pub async fn record_steal(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<StealDto>,
) -> Result<StatusCode, ApiError> {
    if dto.amount <= 0 {
        return Err(DomainError::ValidationError("Le montant doit etre positif".into()).into());
    }

    let mut tx = state
        .pg_pool
        .begin()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Lock + lire le solde de la victime pour ne voler que ce qu'elle a
    let victim = sqlx::query_as::<_, (i64,)>(
        "SELECT coins FROM coude_players WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(&guild_id)
    .bind(&dto.victim_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let (victim_coins,) = victim.ok_or_else(|| ApiError::from(DomainError::NotFound("Victime introuvable".into())))?;

    // Ne voler que le minimum entre le montant demande et le solde reel
    let actual_stolen = dto.amount.min(victim_coins);

    if actual_stolen <= 0 {
        return Err(DomainError::ValidationError("La victime n'a pas de coins a voler".into()).into());
    }

    // Debit victim (montant reel)
    sqlx::query(
        r#"UPDATE coude_players SET coins = coins - $3, total_lost = total_lost + $3, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&dto.victim_id)
    .bind(actual_stolen)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    // Credit thief (meme montant reel — pas de creation de coins)
    sqlx::query(
        r#"UPDATE coude_players SET coins = coins + $3, total_stolen = total_stolen + $3,
        total_earned = total_earned + $3, updated_at = NOW()
        WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(&guild_id)
    .bind(&dto.thief_id)
    .bind(actual_stolen)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

// ── 35-37. Primes ──

/// POST /api/coude/{guild_id}/primes
pub async fn create_prime(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<CreatePrimeDto>,
) -> Result<Json<PrimeDto>, ApiError> {
    let prime = sqlx::query_as::<_, PrimeDto>(
        r#"INSERT INTO coude_primes (guild_id, target_id, target_name, placed_by_id, placed_by_name, amount)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id::text, guild_id, target_id, target_name, placed_by_id, placed_by_name,
            amount, claimed, claimed_by_id, claimed_by_name, claimed_at::text, created_at::text"#,
    )
    .bind(&guild_id)
    .bind(&dto.target_id)
    .bind(&dto.target_name)
    .bind(&dto.placed_by_id)
    .bind(&dto.placed_by_name)
    .bind(dto.amount)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(prime))
}

/// GET /api/coude/{guild_id}/primes/{target_id}/active
pub async fn get_active_primes(
    State(state): State<AppState>,
    Path((guild_id, target_id)): Path<(String, String)>,
) -> Result<Json<Vec<PrimeDto>>, ApiError> {
    let primes = sqlx::query_as::<_, PrimeDto>(
        r#"SELECT id::text, guild_id, target_id, target_name, placed_by_id, placed_by_name,
            amount, claimed, claimed_by_id, claimed_by_name, claimed_at::text, created_at::text
        FROM coude_primes WHERE guild_id = $1 AND target_id = $2 AND claimed = FALSE"#,
    )
    .bind(&guild_id)
    .bind(&target_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(primes))
}

/// POST /api/coude/{guild_id}/primes/claim
pub async fn claim_primes(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<ClaimPrimesDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64,)>(
        r#"WITH claimed AS (
            UPDATE coude_primes SET claimed = TRUE, claimed_by_id = $3, claimed_by_name = $4, claimed_at = NOW()
            WHERE guild_id = $1 AND target_id = $2 AND claimed = FALSE
            RETURNING amount
        )
        SELECT COALESCE(SUM(amount), 0) FROM claimed"#,
    )
    .bind(&guild_id)
    .bind(&dto.target_id)
    .bind(&dto.claimer_id)
    .bind(&dto.claimer_name)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(serde_json::json!({ "total_claimed": row.0 })))
}

// ── 38-40. Insurance ──

/// POST /api/coude/{guild_id}/insurance/buy
pub async fn buy_insurance(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<BuyInsuranceDto>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        r#"INSERT INTO coude_insurances (guild_id, user_id, is_scam, expires_at)
        VALUES ($1, $2, $3, NOW() + INTERVAL '1 hour')"#,
    )
    .bind(&guild_id)
    .bind(&dto.user_id)
    .bind(dto.is_scam)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/coude/{guild_id}/insurance/{user_id}
pub async fn get_active_insurance(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Option<InsuranceDto>>, ApiError> {
    let insurance = sqlx::query_as::<_, InsuranceDto>(
        r#"SELECT id::text, is_scam, expires_at::text
        FROM coude_insurances WHERE guild_id = $1 AND user_id = $2
        AND active = TRUE AND expires_at > NOW()
        ORDER BY expires_at DESC LIMIT 1"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(insurance))
}

/// POST /api/coude/insurance/{insurance_id}/expire
pub async fn expire_insurance(
    State(state): State<AppState>,
    Path(insurance_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query(
        "UPDATE coude_insurances SET active = FALSE, expires_at = NOW() WHERE id = $1::uuid",
    )
    .bind(&insurance_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    if result.rows_affected() == 0 {
        return Err(DomainError::NotFound("Assurance introuvable".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── 41. Leaderboard ──

/// GET /api/coude/{guild_id}/leaderboard/{category}
pub async fn leaderboard(
    State(state): State<AppState>,
    Path((guild_id, category)): Path<(String, String)>,
    Query(params): Query<LeaderboardQueryParams>,
) -> Result<Json<Vec<LeaderboardEntry>>, ApiError> {
    let limit = params.limit.unwrap_or(10).min(100);

    let query = match category.as_str() {
        "richest" => {
            "SELECT user_id, username, coins AS value FROM coude_players WHERE guild_id = $1 ORDER BY coins DESC LIMIT $2"
        }
        "thieves" => {
            "SELECT user_id, username, total_stolen AS value FROM coude_players WHERE guild_id = $1 ORDER BY total_stolen DESC LIMIT $2"
        }
        "cowards" => {
            "SELECT user_id, username, cowardice_count::BIGINT AS value FROM coude_players WHERE guild_id = $1 ORDER BY cowardice_count DESC LIMIT $2"
        }
        "chaos" => {
            "SELECT user_id, username, chaos_events::BIGINT AS value FROM coude_players WHERE guild_id = $1 ORDER BY chaos_events DESC LIMIT $2"
        }
        "level" => {
            "SELECT user_id, username, level::BIGINT AS value FROM coude_players WHERE guild_id = $1 ORDER BY level DESC, xp DESC LIMIT $2"
        }
        _ => {
            return Err(DomainError::ValidationError(format!(
                "Categorie invalide: {}. Valeurs acceptees: richest, thieves, cowards, chaos, level",
                category
            ))
            .into());
        }
    };

    let entries = sqlx::query_as::<_, LeaderboardEntry>(query)
        .bind(&guild_id)
        .bind(limit)
        .fetch_all(&state.pg_pool)
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(entries))
}

// ── 42. Get all guild IDs ──

/// GET /api/coude/guilds
pub async fn get_all_guild_ids(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, ApiError> {
    let rows = sqlx::query_as::<_, (String,)>(
        "SELECT DISTINCT guild_id FROM coude_players",
    )
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let guild_ids: Vec<String> = rows.into_iter().map(|r| r.0).collect();
    Ok(Json(guild_ids))
}

// ── 43. Random players ──

/// GET /api/coude/{guild_id}/players/random?count=2
pub async fn get_random_players(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Query(params): Query<RandomPlayersQuery>,
) -> Result<Json<Vec<FullPlayerDto>>, ApiError> {
    let count = params.count.unwrap_or(2).min(50);

    let players = sqlx::query_as::<_, FullPlayerDto>(
        r#"SELECT guild_id, user_id, username, coins,
            total_wins, total_losses, total_draws,
            total_earned, total_lost, total_stolen,
            cowardice_count, chaos_events, casino_wins, casino_losses,
            level, xp, stat_points, atk, def, class, title,
            hp_current, hp_max, hp_last_regen::text, class_changed_at::text, repos_last_used::text, season,
            created_at::text, updated_at::text
        FROM coude_players WHERE guild_id = $1 AND coins > 50
        ORDER BY RANDOM() LIMIT $2"#,
    )
    .bind(&guild_id)
    .bind(count)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(players))
}

// ── 44. Daily chaos ──

/// POST /api/coude/{guild_id}/daily-chaos
pub async fn log_daily_chaos(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<DailyChaosDto>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        r#"INSERT INTO coude_daily_chaos (guild_id, loser_id, loser_name, winner_id, winner_name, amount)
        VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(&guild_id)
    .bind(&dto.loser_id)
    .bind(&dto.loser_name)
    .bind(&dto.winner_id)
    .bind(&dto.winner_name)
    .bind(dto.amount)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

// ── 45. Active events ──

/// GET /api/coude/{guild_id}/events
pub async fn get_active_events(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<EventDto>>, ApiError> {
    let events = sqlx::query_as::<_, EventDto>(
        r#"SELECT id::text, guild_id, active, expires_at::text, created_at::text
        FROM coude_events WHERE guild_id = $1 AND active = TRUE AND expires_at > NOW()"#,
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(events))
}

// ── 46-49. Inventory ──

/// GET /api/coude/{guild_id}/inventory/{user_id}
pub async fn get_inventory(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<Json<Vec<InventoryItemDto>>, ApiError> {
    let items = sqlx::query_as::<_, InventoryItemDto>(
        "SELECT guild_id, user_id, item_key, quantity FROM coude_inventory WHERE guild_id = $1 AND user_id = $2 AND quantity > 0",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(items))
}

/// POST /api/coude/{guild_id}/inventory/{user_id}/add
pub async fn add_item(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<AddItemDto>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        r#"INSERT INTO coude_inventory (guild_id, user_id, item_key, quantity)
        VALUES ($1, $2, $3, 1)
        ON CONFLICT (guild_id, user_id, item_key)
        DO UPDATE SET quantity = coude_inventory.quantity + 1"#,
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(&dto.item_key)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/coude/{guild_id}/inventory/{user_id}/use
pub async fn use_item(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UseItemDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE coude_inventory SET quantity = quantity - 1 WHERE guild_id = $1 AND user_id = $2 AND item_key = $3 AND quantity > 0",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(&dto.item_key)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(
        serde_json::json!({ "success": result.rows_affected() > 0 }),
    ))
}

/// GET /api/coude/{guild_id}/inventory/{user_id}/has/{item_key}
pub async fn has_item(
    State(state): State<AppState>,
    Path((guild_id, user_id, item_key)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM coude_inventory WHERE guild_id = $1 AND user_id = $2 AND item_key = $3 AND quantity > 0",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(&item_key)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(Json(serde_json::json!({ "has_item": row.0 > 0 })))
}

// ── 50. Update HP ──

#[derive(Debug, Deserialize)]
pub struct UpdateHpDto {
    pub hp_current: i32,
    pub hp_max: i32,
}

/// POST /api/coude/{guild_id}/players/{user_id}/hp
pub async fn update_hp(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
    Json(dto): Json<UpdateHpDto>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        "UPDATE coude_players SET hp_current = $3, hp_max = $4, hp_last_regen = NOW(), updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .bind(dto.hp_current)
    .bind(dto.hp_max)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

// ── 51. Repos (full heal) ──

/// POST /api/coude/{guild_id}/players/{user_id}/repos
pub async fn repos(
    State(state): State<AppState>,
    Path((guild_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        "UPDATE coude_players SET hp_current = hp_max, repos_last_used = NOW(), hp_last_regen = NOW(), updated_at = NOW() WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(&guild_id)
    .bind(&user_id)
    .execute(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    Ok(StatusCode::NO_CONTENT)
}

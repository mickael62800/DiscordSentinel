use axum::extract::{Path, Query, State};
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

// ── Handlers ──

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
    let _ = sqlx::query(
        "UPDATE coude_bets SET won = false WHERE combat_id = $1::uuid AND won IS NULL"
    )
    .bind(&combat_id)
    .execute(&state.pg_pool)
    .await;

    Ok(ok_response())
}

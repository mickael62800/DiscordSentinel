//! Handlers HTTP pour le tournoi hebdomadaire (migration 139).
//!
//! Endpoints minimalistes :
//!   - GET /api/coude/{guild_id}/tournaments/current
//!   - GET /api/coude/{guild_id}/tournaments/history
//!
//! Pragmatique : on lit directement le pool via `state.pg_pool` plutot
//! que de passer par un use case / repository dedie. La logique metier
//! (resolution, distribution du prix) vit dans le coude-worker, pas ici.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

#[derive(Debug, Serialize)]
pub struct StandingDto {
    pub user_id: String,
    pub username: String,
    pub net_gain: i64,
    pub rank: i32,
}

#[derive(Debug, Serialize)]
pub struct CurrentTournamentDto {
    pub guild_id: String,
    pub week_start: DateTime<Utc>,
    pub week_end: DateTime<Utc>,
    pub prize_pool_estimated: i64,
    pub standings: Vec<StandingDto>,
}

#[derive(Debug, Serialize)]
pub struct PastTournamentDto {
    pub id: String,
    pub guild_id: String,
    pub week_start: DateTime<Utc>,
    pub week_end: DateTime<Utc>,
    pub winner_user_id: Option<String>,
    pub winner_username: Option<String>,
    pub winner_net_gain: i64,
    pub prize_amount: i64,
    pub status: String,
    pub resolved_at: Option<DateTime<Utc>>,
}

// Bornes de semaine + prize_pool : regles metier extraites vers
// `domain/entities/coude_tournament.rs` (purement testables).
use crate::domain::entities::{current_week_bounds, estimate_tournament_prize_pool};

/// GET /api/coude/{guild_id}/tournaments/current
pub async fn get_current_tournament(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<CurrentTournamentDto>, ApiError> {
    let (week_start, week_end) = current_week_bounds();

    // Sum des wallet_transactions par user sur la semaine.
    // Positif = net gain sur la periode.
    let rows = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT wt.user_id, COALESCE(SUM(wt.amount), 0)::BIGINT AS net
        FROM wallet_transactions wt
        WHERE wt.guild_id = $1
          AND wt.created_at >= $2
          AND wt.created_at <= $3
        GROUP BY wt.user_id
        ORDER BY net DESC
        LIMIT 10
        "#,
    )
    .bind(&guild_id)
    .bind(week_start)
    .bind(week_end)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(format!("tournaments query: {e}"))))?;

    // Lookup usernames via user_wallets.
    let mut standings = Vec::with_capacity(rows.len());
    for (idx, (user_id, net)) in rows.into_iter().enumerate() {
        let username: Option<String> = sqlx::query_scalar(
            "SELECT username FROM user_wallets WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(&guild_id)
        .bind(&user_id)
        .fetch_optional(&state.pg_pool)
        .await
        .map_err(|e| ApiError::from(DomainError::Internal(format!("username lookup: {e}"))))?;

        standings.push(StandingDto {
            user_id,
            username: username.unwrap_or_else(|| "?".to_string()),
            net_gain: net,
            rank: (idx + 1) as i32,
        });
    }

    // Prize pool estime : 10% de la caisse communautaire par defaut.
    // Pragmatique : on ne lit pas la config ici, on utilise le default.
    let cashbox: Option<i64> = sqlx::query_scalar(
        "SELECT balance FROM coude_cashbox WHERE guild_id = $1",
    )
    .bind(&guild_id)
    .fetch_optional(&state.pg_pool)
    .await
    .ok()
    .flatten();

    let prize_pool_estimated = estimate_tournament_prize_pool(cashbox);

    Ok(Json(CurrentTournamentDto {
        guild_id,
        week_start,
        week_end,
        prize_pool_estimated,
        standings,
    }))
}

/// GET /api/coude/{guild_id}/tournaments/history
pub async fn get_tournament_history(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<PastTournamentDto>>, ApiError> {
    let rows = sqlx::query_as::<_, (
        sqlx::types::Uuid,
        String,
        DateTime<Utc>,
        DateTime<Utc>,
        Option<String>,
        Option<String>,
        Option<i64>,
        i64,
        String,
        Option<DateTime<Utc>>,
    )>(
        r#"
        SELECT id, guild_id, week_start, week_end, winner_user_id,
               winner_username, winner_net_gain, prize_amount, status, resolved_at
        FROM coude_weekly_tournaments
        WHERE guild_id = $1
        ORDER BY week_start DESC
        LIMIT 20
        "#,
    )
    .bind(&guild_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(|e| ApiError::from(DomainError::Internal(format!("history query: {e}"))))?;

    let out = rows
        .into_iter()
        .map(
            |(id, guild_id, week_start, week_end, winner_user_id, winner_username, winner_net_gain, prize_amount, status, resolved_at)| {
                PastTournamentDto {
                    id: id.to_string(),
                    guild_id,
                    week_start,
                    week_end,
                    winner_user_id,
                    winner_username,
                    winner_net_gain: winner_net_gain.unwrap_or(0),
                    prize_amount,
                    status,
                    resolved_at,
                }
            },
        )
        .collect();

    Ok(Json(out))
}

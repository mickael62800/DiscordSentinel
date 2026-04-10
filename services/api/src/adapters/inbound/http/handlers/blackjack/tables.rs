//! Handlers multiplayer : tables, joueurs, clôture, listing des parties.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use super::dto::{CreateTableDto, JoinTableDto, TableDto, TablePlayerDto};
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::state::AppState;
use crate::domain::errors::DomainError;

fn pg_err(e: sqlx::Error) -> ApiError {
    ApiError::from(DomainError::Internal(e.to_string()))
}

/// POST /api/blackjack/tables — crée une table multijoueur avec sabot de 6 decks.
pub async fn create_table(
    State(state): State<AppState>,
    Json(dto): Json<CreateTableDto>,
) -> Result<Json<TableDto>, ApiError> {
    use crate::domain::entities::create_deck;
    use rand::seq::SliceRandom;

    // Créer un sabot de 6 decks mélangés (312 cartes)
    let mut shoe: Vec<crate::domain::entities::Card> = Vec::with_capacity(312);
    for _ in 0..6 {
        shoe.extend(create_deck());
    }
    shoe.shuffle(&mut rand::thread_rng());

    let shoe_json = serde_json::to_value(&shoe)
        .map_err(|e| ApiError::from(DomainError::Internal(e.to_string())))?;

    let table = sqlx::query_as::<_, TableDto>(
        r#"INSERT INTO blackjack_tables (guild_id, channel_id, owner_id, owner_name, deck)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id::text, guild_id, channel_id, owner_id, owner_name, status, created_at::text"#,
    )
    .bind(&dto.guild_id)
    .bind(&dto.channel_id)
    .bind(&dto.owner_id)
    .bind(&dto.owner_name)
    .bind(&shoe_json)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(pg_err)?;

    // Le owner est automatiquement joueur
    sqlx::query(
        "INSERT INTO blackjack_table_players (table_id, user_id, user_name)
         VALUES ($1::uuid, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(&table.id)
    .bind(&dto.owner_id)
    .bind(&dto.owner_name)
    .execute(&state.pg_pool)
    .await
    .ok();

    Ok(Json(table))
}

/// POST /api/blackjack/tables/{table_id}/join — rejoindre une table.
pub async fn join_table(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
    Json(dto): Json<JoinTableDto>,
) -> Result<StatusCode, ApiError> {
    // Vérifier que la table est ouverte + récupérer le guild_id
    let table_info = sqlx::query_as::<_, (String, String)>(
        "SELECT status, guild_id FROM blackjack_tables WHERE id = $1::uuid",
    )
    .bind(&table_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(pg_err)?;

    match &table_info {
        Some((s, _)) if s == "open" => {}
        Some(_) => return Err(DomainError::Conflict("Table fermee".into()).into()),
        None => return Err(DomainError::NotFound("Table introuvable".into()).into()),
    }

    let guild_id = table_info.unwrap().1;

    // Vérifier la limite de joueurs
    let current_count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(*) FROM blackjack_table_players WHERE table_id = $1::uuid",
    )
    .bind(&table_id)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(pg_err)?
    .0;

    let max_players: i64 = {
        let config = state
            .bot_config_repo
            .get_config(&guild_id, "blackjack-bot")
            .await
            .unwrap_or_default();
        config
            .iter()
            .find(|c| c.config_key == "max_players_per_table")
            .and_then(|c| c.config_value.parse().ok())
            .unwrap_or(7)
    };

    if current_count >= max_players {
        return Err(DomainError::ValidationError(format!(
            "Table pleine ({}/{} joueurs)",
            current_count, max_players
        ))
        .into());
    }

    sqlx::query(
        "INSERT INTO blackjack_table_players (table_id, user_id, user_name)
         VALUES ($1::uuid, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(&table_id)
    .bind(&dto.user_id)
    .bind(&dto.user_name)
    .execute(&state.pg_pool)
    .await
    .map_err(pg_err)?;

    // Mettre à jour last_activity
    sqlx::query("UPDATE blackjack_tables SET last_activity = NOW() WHERE id = $1::uuid")
        .bind(&table_id)
        .execute(&state.pg_pool)
        .await
        .ok();

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/blackjack/tables/{table_id}/players — lister les joueurs d'une table.
pub async fn list_table_players(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
) -> Result<Json<Vec<TablePlayerDto>>, ApiError> {
    let players = sqlx::query_as::<_, TablePlayerDto>(
        "SELECT user_id, user_name, joined_at::text FROM blackjack_table_players
         WHERE table_id = $1::uuid ORDER BY joined_at",
    )
    .bind(&table_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(pg_err)?;

    Ok(Json(players))
}

/// GET /api/blackjack/tables/by-channel/{channel_id} — trouver la table par channel.
pub async fn get_table_by_channel(
    State(state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Result<Json<Option<TableDto>>, ApiError> {
    let table = sqlx::query_as::<_, TableDto>(
        r#"SELECT id::text, guild_id, channel_id, owner_id, owner_name, status, created_at::text
           FROM blackjack_tables WHERE channel_id = $1 AND status = 'open'"#,
    )
    .bind(&channel_id)
    .fetch_optional(&state.pg_pool)
    .await
    .map_err(pg_err)?;

    Ok(Json(table))
}

/// DELETE /api/blackjack/tables/{table_id} — fermer une table.
pub async fn close_table(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    sqlx::query(
        "UPDATE blackjack_tables SET status = 'closed' WHERE id = $1::uuid AND status = 'open'",
    )
    .bind(&table_id)
    .execute(&state.pg_pool)
    .await
    .map_err(pg_err)?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/blackjack/tables/{table_id}/games — résumé des parties d'une table.
pub async fn list_table_games(
    State(state): State<AppState>,
    Path(table_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64)>(
        r#"SELECT id::text, user_id, username, status, bet, payout
           FROM blackjack_games WHERE table_id = $1::uuid ORDER BY created_at DESC"#,
    )
    .bind(&table_id)
    .fetch_all(&state.pg_pool)
    .await
    .map_err(pg_err)?;

    let result: Vec<serde_json::Value> = rows
        .iter()
        .map(|(id, uid, name, status, bet, payout)| {
            serde_json::json!({
                "id": id, "user_id": uid, "username": name,
                "status": status, "bet": bet, "payout": payout,
            })
        })
        .collect();

    Ok(Json(result))
}

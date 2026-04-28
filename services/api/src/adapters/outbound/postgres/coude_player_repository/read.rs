//! Methodes de lecture du `CoudePlayerRepository` : get/list/random.
//!
//! Sous-module du repository Postgres coude_players (refactor 2026-04
//! du god-object 834 LOC). Chaque methode est une free function qui
//! prend `&PgCoudePlayerRepository` en argument, appelee par la thin
//! trait impl dans `mod.rs`.

use crate::domain::entities::coude::player::CoudePlayer;
use crate::domain::errors::DomainError;

use super::pg_err;
use super::PgCoudePlayerRepository;
use super::PlayerRow;
use super::PLAYER_COLUMNS;
pub(super) async fn get_or_create(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    username: &str,
) -> Result<CoudePlayer, DomainError> {
    // 1. Creer/mettre a jour le joueur coude
    sqlx::query(
        r#"INSERT INTO coude_players (guild_id, user_id, username)
           VALUES ($1, $2, $3)
           ON CONFLICT (guild_id, user_id)
           DO UPDATE SET username = EXCLUDED.username, updated_at = NOW()"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(username)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;

    // 2. Auto-creer le wallet partage si absent (starting_coins = 200).
    let starting_coins: i64 = std::env::var("WALLET_STARTING_COINS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    sqlx::query(
        r#"INSERT INTO user_wallets (id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, $2, $3, $4, $4, 0, NOW(), NOW())
           ON CONFLICT (guild_id, user_id) DO NOTHING"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(username)
    .bind(starting_coins)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;

    // 3. Re-fetch avec le PLAYER_COLUMNS qui lit coins depuis user_wallets
    let sql = format!(
        "SELECT {cols} FROM coude_players cp WHERE cp.guild_id = $1 AND cp.user_id = $2",
        cols = PLAYER_COLUMNS
    );
    let row: PlayerRow = sqlx::query_as(&sql)
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&repo.pool)
        .await
        .map_err(pg_err)?;
    Ok(row.into())
}

pub(super) async fn get(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<Option<CoudePlayer>, DomainError> {
    let sql = format!(
        "SELECT {cols} FROM coude_players cp WHERE cp.guild_id = $1 AND cp.user_id = $2",
        cols = PLAYER_COLUMNS
    );
    let row: Option<PlayerRow> = sqlx::query_as(&sql)
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&repo.pool)
        .await
        .map_err(pg_err)?;
    Ok(row.map(Into::into))
}

pub(super) async fn list(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    limit: i64,
) -> Result<Vec<CoudePlayer>, DomainError> {
    // Phase 2 A.2 — Lit depuis la vue materialisee `mv_coude_leaderboard`
    // refreshee toutes les 5 min par le cache-worker. La MV contient toutes
    // les colonnes de coude_players + un `rank` precalcule, donc on garde
    // le meme PLAYER_COLUMNS et on remplace juste FROM + ORDER BY.
    // Staleness max 5 min — acceptable pour une UI listing.
    let sql = format!(
        r#"SELECT {cols}
           FROM mv_coude_leaderboard cp
           WHERE cp.guild_id = $1
           ORDER BY rank
           LIMIT $2"#,
        cols = PLAYER_COLUMNS
    );
    let rows: Vec<PlayerRow> = sqlx::query_as(&sql)
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&repo.pool)
        .await
        .map_err(pg_err)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub(super) async fn random_active(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    count: i64,
    min_coins: i64,
) -> Result<Vec<CoudePlayer>, DomainError> {
    let sql = format!(
        r#"SELECT {cols}
           FROM coude_players cp
           WHERE cp.guild_id = $1
             AND COALESCE((SELECT w.coins FROM user_wallets w WHERE w.guild_id = cp.guild_id AND w.user_id = cp.user_id), 0) > $2
           ORDER BY RANDOM()
           LIMIT $3"#,
        cols = PLAYER_COLUMNS
    );
    let rows: Vec<PlayerRow> = sqlx::query_as(&sql)
        .bind(guild_id)
        .bind(min_coins)
        .bind(count)
        .fetch_all(&repo.pool)
        .await
        .map_err(pg_err)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub(super) async fn list_guild_ids(
    repo: &PgCoudePlayerRepository,
) -> Result<Vec<String>, DomainError> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT DISTINCT guild_id FROM coude_players")
        .fetch_all(&repo.pool)
        .await
        .map_err(pg_err)?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

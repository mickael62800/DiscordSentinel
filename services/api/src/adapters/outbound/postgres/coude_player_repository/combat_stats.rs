//! Enregistrement des resultats de combat et compteurs : win/loss/draw,
//! cowardice, chaos.

use crate::adapters::outbound::postgres::wallet_tx_log::log_wallet_tx;
use crate::domain::errors::DomainError;

use super::{pg_err, PgCoudePlayerRepository};

pub(super) async fn record_win(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    earned: i64,
    stolen: i64,
) -> Result<bool, DomainError> {
    let mut tx = repo.pool.begin().await.map_err(pg_err)?;
    sqlx::query(
        r#"UPDATE coude_players
           SET total_wins = total_wins + 1,
               total_earned = total_earned + $3,
               total_stolen = total_stolen + $4,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(guild_id).bind(user_id).bind(earned).bind(stolen)
    .execute(&mut *tx).await.map_err(pg_err)?;

    let row: Option<i64> = sqlx::query_scalar(
        "UPDATE user_wallets SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins",
    )
    .bind(guild_id).bind(user_id).bind(earned)
    .fetch_optional(&mut *tx).await.map_err(pg_err)?;
    let Some(balance_after) = row else {
        tx.commit().await.map_err(pg_err)?;
        return Ok(false);
    };
    log_wallet_tx(&mut tx, guild_id, user_id, earned, balance_after, "coude_combat_win", "Combat gagne").await?;
    tx.commit().await.map_err(pg_err)?;
    Ok(true)
}

pub(super) async fn record_loss(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    lost: i64,
) -> Result<bool, DomainError> {
    let mut tx = repo.pool.begin().await.map_err(pg_err)?;
    sqlx::query(
        r#"UPDATE coude_players
           SET total_losses = total_losses + 1,
               total_lost = total_lost + $3,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(guild_id).bind(user_id).bind(lost)
    .execute(&mut *tx).await.map_err(pg_err)?;

    let row: Option<i64> = sqlx::query_scalar(
        "UPDATE user_wallets SET coins = GREATEST(0, coins - $3), total_spent = total_spent + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins",
    )
    .bind(guild_id).bind(user_id).bind(lost)
    .fetch_optional(&mut *tx).await.map_err(pg_err)?;
    let Some(balance_after) = row else {
        tx.commit().await.map_err(pg_err)?;
        return Ok(false);
    };
    log_wallet_tx(&mut tx, guild_id, user_id, -lost, balance_after, "coude_combat_loss", "Combat perdu").await?;
    tx.commit().await.map_err(pg_err)?;
    Ok(true)
}

pub(super) async fn record_draw(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    lost: i64,
) -> Result<bool, DomainError> {
    let mut tx = repo.pool.begin().await.map_err(pg_err)?;
    sqlx::query(
        "UPDATE coude_players SET total_draws = total_draws + 1, total_lost = total_lost + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id).bind(user_id).bind(lost)
    .execute(&mut *tx).await.map_err(pg_err)?;

    let row: Option<i64> = sqlx::query_scalar(
        "UPDATE user_wallets SET coins = GREATEST(0, coins - $3), total_spent = total_spent + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins",
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(lost)
    .fetch_optional(&mut *tx)
    .await
    .map_err(pg_err)?;
    let Some(balance_after) = row else {
        tx.commit().await.map_err(pg_err)?;
        return Ok(false);
    };
    log_wallet_tx(&mut tx, guild_id, user_id, -lost, balance_after, "coude_combat_draw", "Combat egalite").await?;
    tx.commit().await.map_err(pg_err)?;
    Ok(true)
}

pub(super) async fn increment_cowardice(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<Option<i32>, DomainError> {
    let row: Option<(i32,)> = sqlx::query_as(
        r#"UPDATE coude_players
           SET cowardice_count = cowardice_count + 1, updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2
           RETURNING cowardice_count"#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(row.map(|r| r.0))
}

pub(super) async fn increment_chaos(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
) -> Result<bool, DomainError> {
    let result = sqlx::query(
        "UPDATE coude_players
         SET chaos_events = chaos_events + 1, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(&repo.pool)
    .await
    .map_err(pg_err)?;
    Ok(result.rows_affected() > 0)
}

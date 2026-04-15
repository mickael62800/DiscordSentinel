//! Ajustements wallet : adjust_coins, record_coins_earned, record_coins_lost.

use crate::adapters::outbound::postgres::wallet_tx_log::log_wallet_tx;
use crate::domain::errors::DomainError;

use super::{pg_err, PgCoudePlayerRepository};

pub(super) async fn adjust_coins(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    delta: i64,
) -> Result<bool, DomainError> {
    let mut tx = repo.pool.begin().await.map_err(pg_err)?;
    let row: Option<i64> = sqlx::query_scalar(
        "UPDATE user_wallets
         SET coins = GREATEST(0, coins + $1), updated_at = NOW()
         WHERE guild_id = $2 AND user_id = $3
         RETURNING coins",
    )
    .bind(delta)
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(pg_err)?;
    let Some(balance_after) = row else {
        tx.commit().await.map_err(pg_err)?;
        return Ok(false);
    };
    log_wallet_tx(&mut tx, guild_id, user_id, delta, balance_after, "coude_adjust", "Ajustement manuel").await?;
    tx.commit().await.map_err(pg_err)?;
    Ok(true)
}

pub(super) async fn record_coins_earned(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    amount: i64,
) -> Result<bool, DomainError> {
    let mut tx = repo.pool.begin().await.map_err(pg_err)?;
    sqlx::query(
        "UPDATE coude_players SET total_earned = total_earned + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id).bind(user_id).bind(amount)
    .execute(&mut *tx).await.map_err(pg_err)?;

    let row: Option<i64> = sqlx::query_scalar(
        "UPDATE user_wallets SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins",
    )
    .bind(guild_id).bind(user_id).bind(amount)
    .fetch_optional(&mut *tx).await.map_err(pg_err)?;
    let Some(balance_after) = row else {
        tx.commit().await.map_err(pg_err)?;
        return Ok(false);
    };
    log_wallet_tx(&mut tx, guild_id, user_id, amount, balance_after, "coude_earn", "Gain coude").await?;
    tx.commit().await.map_err(pg_err)?;
    Ok(true)
}

pub(super) async fn record_coins_lost(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    amount: i64,
) -> Result<bool, DomainError> {
    let mut tx = repo.pool.begin().await.map_err(pg_err)?;
    sqlx::query(
        "UPDATE coude_players SET total_lost = total_lost + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id).bind(user_id).bind(amount)
    .execute(&mut *tx).await.map_err(pg_err)?;

    let row: Option<i64> = sqlx::query_scalar(
        "UPDATE user_wallets SET coins = GREATEST(0, coins - $3), total_spent = total_spent + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins",
    )
    .bind(guild_id).bind(user_id).bind(amount)
    .fetch_optional(&mut *tx).await.map_err(pg_err)?;
    let Some(balance_after) = row else {
        tx.commit().await.map_err(pg_err)?;
        return Ok(false);
    };
    log_wallet_tx(&mut tx, guild_id, user_id, -amount, balance_after, "coude_loss", "Perte coude").await?;
    tx.commit().await.map_err(pg_err)?;
    Ok(true)
}

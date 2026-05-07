//! Increments stats-only sur `coude_players` : `total_earned` / `total_lost`.
//!
//! Migration wallet unifie (finale) : ces methodes NE mutent PLUS
//! `user_wallets`. Les mouvements de coins + log `wallet_transactions`
//! sont a la charge de l'appelant, qui doit passer par
//! `ManageWalletUseCase::credit/debit` (ou `WalletRepository::credit/debit`
//! pour les call sites qui ne peuvent pas injecter le use case).
//!
//! `adjust_coins` (ajustement admin) a ete supprime : les handlers HTTP/
//! gRPC delegent desormais directement au `ManageWalletUseCase`.

use sentinel_core::domain::errors::DomainError;

use super::super::super::pg_err;
use super::PgPlayerRepository;
pub(super) async fn record_coins_earned(
    repo: &PgPlayerRepository,
    guild_id: &str,
    user_id: &str,
    amount: i64,
) -> Result<bool, DomainError> {
    let result = sqlx::query(
        "UPDATE coude_players SET total_earned = total_earned + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id).bind(user_id).bind(amount)
    .execute(&repo.pool).await.map_err(pg_err)?;
    Ok(result.rows_affected() > 0)
}

pub(super) async fn record_coins_lost(
    repo: &PgPlayerRepository,
    guild_id: &str,
    user_id: &str,
    amount: i64,
) -> Result<bool, DomainError> {
    let result = sqlx::query(
        "UPDATE coude_players SET total_lost = total_lost + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id).bind(user_id).bind(amount)
    .execute(&repo.pool).await.map_err(pg_err)?;
    Ok(result.rows_affected() > 0)
}

//! Enregistrement des resultats de combat et compteurs : win/loss/draw,
//! cowardice, chaos.
//!
//! Migration #3 (wallet unifie) : ces methodes ne mutent PLUS `user_wallets`.
//! Elles se contentent d'incrementer les compteurs `coude_players`. Les
//! mouvements de coins (credit/debit + wallet_transactions) sont a la charge
//! de l'appelant, qui doit passer par `WalletRepository::credit/debit` ou
//! `ManageWalletUseCase`. Ceci corrige le double-comptage historique (les
//! resolve services faisaient deja les credit/debit + ces fonctions les
//! refaisaient).

use crate::domain::errors::DomainError;

use super::super::super::pg_err;
use super::PgCoudePlayerRepository;
pub(super) async fn record_win(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    earned: i64,
    stolen: i64,
) -> Result<bool, DomainError> {
    let result = sqlx::query(
        r#"UPDATE coude_players
           SET total_wins = total_wins + 1,
               total_earned = total_earned + $3,
               total_stolen = total_stolen + $4,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(guild_id).bind(user_id).bind(earned).bind(stolen)
    .execute(&repo.pool).await.map_err(pg_err)?;
    Ok(result.rows_affected() > 0)
}

pub(super) async fn record_loss(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    lost: i64,
) -> Result<bool, DomainError> {
    let result = sqlx::query(
        r#"UPDATE coude_players
           SET total_losses = total_losses + 1,
               total_lost = total_lost + $3,
               updated_at = NOW()
           WHERE guild_id = $1 AND user_id = $2"#,
    )
    .bind(guild_id).bind(user_id).bind(lost)
    .execute(&repo.pool).await.map_err(pg_err)?;
    Ok(result.rows_affected() > 0)
}

pub(super) async fn record_draw(
    repo: &PgCoudePlayerRepository,
    guild_id: &str,
    user_id: &str,
    lost: i64,
) -> Result<bool, DomainError> {
    let result = sqlx::query(
        "UPDATE coude_players SET total_draws = total_draws + 1, total_lost = total_lost + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2",
    )
    .bind(guild_id).bind(user_id).bind(lost)
    .execute(&repo.pool).await.map_err(pg_err)?;
    Ok(result.rows_affected() > 0)
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

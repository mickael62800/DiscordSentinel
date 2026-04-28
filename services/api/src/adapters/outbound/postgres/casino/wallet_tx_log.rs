use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use crate::domain::errors::DomainError;

/// Insere une ligne dans `wallet_transactions` au sein de la transaction courante.
///
/// `signed_amount` : positif pour un credit, negatif pour un debit.
/// `balance_after` : solde resultant du wallet apres l'operation (a lire via
/// un `RETURNING coins` sur l'UPDATE precedent).
///
/// Utilise par les repos coude (economy, bet, player) et les jobs du
/// coude-worker pour garantir que chaque mutation de `user_wallets` produit
/// une ligne de ledger consultable par la commande `/resume`.
pub async fn log_wallet_tx(
    tx: &mut Transaction<'_, Postgres>,
    guild_id: &str,
    user_id: &str,
    signed_amount: i64,
    balance_after: i64,
    source: &str,
    description: &str,
) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO wallet_transactions \
            (id, guild_id, user_id, amount, balance_after, source, description, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(guild_id)
    .bind(user_id)
    .bind(signed_amount)
    .bind(balance_after)
    .bind(source)
    .bind(description)
    .execute(&mut **tx)
    .await
    .map_err(|e| DomainError::Internal(format!("log_wallet_tx: {e}")))?;
    Ok(())
}

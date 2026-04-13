use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

/// Credite un wallet et enregistre la ligne correspondante dans
/// `wallet_transactions`. Fire-and-forget : en cas d'erreur on log un warn
/// mais on n'abandonne pas le combat (coherent avec le reste du worker).
/// Retourne `Some(balance_after)` en cas de succes, `None` sinon.
pub async fn credit_and_log(
    pool: &PgPool,
    guild_id: &str,
    user_id: &str,
    amount: i64,
    bump_total_earned: bool,
    source: &str,
    description: &str,
) -> Option<i64> {
    let sql = if bump_total_earned {
        "UPDATE user_wallets SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins"
    } else {
        "UPDATE user_wallets SET coins = coins + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins"
    };
    let balance_after: Option<i64> = match sqlx::query_scalar(sql)
        .bind(guild_id).bind(user_id).bind(amount)
        .fetch_optional(pool).await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, user_id, amount, "Echec credit wallet");
            return None;
        }
    };
    let Some(balance_after) = balance_after else {
        warn!(user_id, "Wallet introuvable pour credit");
        return None;
    };
    log_tx(pool, guild_id, user_id, amount, balance_after, source, description).await;
    Some(balance_after)
}

/// Debite un wallet (clampe a 0) et enregistre la ligne correspondante.
pub async fn debit_and_log(
    pool: &PgPool,
    guild_id: &str,
    user_id: &str,
    amount: i64,
    source: &str,
    description: &str,
) -> Option<i64> {
    let balance_after: Option<i64> = match sqlx::query_scalar(
        "UPDATE user_wallets SET coins = GREATEST(0, coins - $3), total_spent = total_spent + $3, updated_at = NOW()
         WHERE guild_id = $1 AND user_id = $2
         RETURNING coins",
    )
    .bind(guild_id).bind(user_id).bind(amount)
    .fetch_optional(pool).await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, user_id, amount, "Echec debit wallet");
            return None;
        }
    };
    let Some(balance_after) = balance_after else {
        warn!(user_id, "Wallet introuvable pour debit");
        return None;
    };
    log_tx(pool, guild_id, user_id, -amount, balance_after, source, description).await;
    Some(balance_after)
}

async fn log_tx(
    pool: &PgPool,
    guild_id: &str,
    user_id: &str,
    signed_amount: i64,
    balance_after: i64,
    source: &str,
    description: &str,
) {
    if let Err(e) = sqlx::query(
        "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(guild_id)
    .bind(user_id)
    .bind(signed_amount)
    .bind(balance_after)
    .bind(source)
    .bind(description)
    .execute(pool)
    .await
    {
        warn!(error = %e, user_id, signed_amount, source, "Echec INSERT wallet_transactions");
    }
}

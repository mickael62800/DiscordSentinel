//! Adapter Postgres du port `WalletRepository`.

use async_trait::async_trait;
use nexus_core::domain::entities::wallet::Wallet;
use nexus_core::domain::entities::wallet::WalletMutation;
use nexus_core::domain::errors::DomainError;
use nexus_core::ports::outbound::wallet_repository::WalletRepository;
use sqlx::PgPool;

use super::pg_err;

pub struct PgWalletRepository {
    pool: PgPool,
}

impl PgWalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WalletRepository for PgWalletRepository {
    async fn get_or_default(&self, guild_id: &str, user_id: &str) -> Result<Wallet, DomainError> {
        let row: Option<(i64, i64, i64)> = sqlx::query_as(
            "SELECT coins, total_earned, total_spent
             FROM nexus_wallets WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        let mut wallet = Wallet::new(guild_id, user_id);
        if let Some((coins, earned, spent)) = row {
            wallet.coins = coins;
            wallet.total_earned = earned;
            wallet.total_spent = spent;
        }
        Ok(wallet)
    }

    async fn save_with_transaction(
        &self,
        wallet: &Wallet,
        mutation: &WalletMutation,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        sqlx::query(
            "INSERT INTO nexus_wallets (guild_id, user_id, coins, total_earned, total_spent)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (guild_id, user_id) DO UPDATE SET
                coins = EXCLUDED.coins,
                total_earned = EXCLUDED.total_earned,
                total_spent = EXCLUDED.total_spent,
                updated_at = NOW()",
        )
        .bind(&wallet.guild_id)
        .bind(&wallet.user_id)
        .bind(wallet.coins)
        .bind(wallet.total_earned)
        .bind(wallet.total_spent)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        sqlx::query(
            "INSERT INTO nexus_wallet_transactions
             (guild_id, user_id, amount, balance_after, source, description)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&wallet.guild_id)
        .bind(&wallet.user_id)
        .bind(mutation.amount)
        .bind(mutation.balance_after)
        .bind(&mutation.source)
        .bind(&mutation.description)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)
    }
}

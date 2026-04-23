use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use tracing::info;

use crate::domain::entities::{Wallet, WalletTransaction};
use crate::domain::errors::DomainError;
use crate::ports::outbound::WalletRepository;

pub struct PgWalletRepository {
    pool: PgPool,
}

impl PgWalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct WalletRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    coins: i64,
    total_earned: i64,
    total_spent: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<WalletRow> for Wallet {
    fn from(r: WalletRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            username: r.username,
            coins: r.coins,
            total_earned: r.total_earned,
            total_spent: r.total_spent,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WalletTransactionRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    amount: i64,
    balance_after: i64,
    source: String,
    description: String,
    created_at: DateTime<Utc>,
}

impl From<WalletTransactionRow> for WalletTransaction {
    fn from(r: WalletTransactionRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            amount: r.amount,
            balance_after: r.balance_after,
            source: r.source,
            description: r.description,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl WalletRepository for PgWalletRepository {
    async fn get_or_create(&self, guild_id: &str, user_id: &str, username: &str, starting_coins: i64) -> Result<Wallet, DomainError> {
        let row = sqlx::query_as::<_, WalletRow>(
            r#"INSERT INTO user_wallets (id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, 0, 0, NOW(), NOW())
               ON CONFLICT (guild_id, user_id) DO UPDATE SET username = EXCLUDED.username, updated_at = NOW()
               RETURNING id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at"#,
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(starting_coins)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("get_or_create wallet: {e}")))?;

        Ok(Wallet::from(row))
    }

    async fn get(&self, guild_id: &str, user_id: &str) -> Result<Option<Wallet>, DomainError> {
        let row = sqlx::query_as::<_, WalletRow>(
            "SELECT id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at
             FROM user_wallets WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("get wallet: {e}")))?;

        Ok(row.map(Wallet::from))
    }

    async fn credit(&self, guild_id: &str, user_id: &str, amount: i64, source: &str, description: &str) -> Result<Wallet, DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Internal(format!("credit begin tx: {e}")))?;

        let row = sqlx::query_as::<_, WalletRow>(
            r#"UPDATE user_wallets
               SET coins = coins + $1, total_earned = total_earned + $1, updated_at = NOW()
               WHERE guild_id = $2 AND user_id = $3
               RETURNING id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at"#,
        )
        .bind(amount)
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("credit update: {e}")))?
        .ok_or_else(|| DomainError::NotFound("Portefeuille introuvable".into()))?;

        let wallet = Wallet::from(row);

        sqlx::query(
            "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(user_id)
        .bind(amount)
        .bind(wallet.coins)
        .bind(source)
        .bind(description)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("credit insert tx: {e}")))?;

        tx.commit().await
            .map_err(|e| DomainError::Internal(format!("credit commit: {e}")))?;

        info!(guild_id, user_id, amount, balance = wallet.coins, source, "Wallet credit");
        Ok(wallet)
    }

    async fn debit(&self, guild_id: &str, user_id: &str, amount: i64, source: &str, description: &str) -> Result<Wallet, DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Internal(format!("debit begin tx: {e}")))?;

        // Verifier le solde avant debit
        let current = sqlx::query_as::<_, WalletRow>(
            "SELECT id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at
             FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("debit select: {e}")))?
        .ok_or_else(|| DomainError::NotFound("Portefeuille introuvable".into()))?;

        if current.coins < amount {
            return Err(DomainError::ValidationError(format!(
                "Solde insuffisant : tu as {} coins, il en faut {} (manque {}). Reduis ta mise ou gagne des coins avant de rejouer.",
                current.coins,
                amount,
                amount - current.coins
            )));
        }

        let row = sqlx::query_as::<_, WalletRow>(
            r#"UPDATE user_wallets
               SET coins = coins - $1, total_spent = total_spent + $1, updated_at = NOW()
               WHERE guild_id = $2 AND user_id = $3
               RETURNING id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at"#,
        )
        .bind(amount)
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("debit update: {e}")))?;

        let wallet = Wallet::from(row);

        sqlx::query(
            "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(user_id)
        .bind(-amount)
        .bind(wallet.coins)
        .bind(source)
        .bind(description)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("debit insert tx: {e}")))?;

        tx.commit().await
            .map_err(|e| DomainError::Internal(format!("debit commit: {e}")))?;

        info!(guild_id, user_id, amount, balance = wallet.coins, source, "Wallet debit");
        Ok(wallet)
    }

    async fn transfer(&self, guild_id: &str, from_user: &str, to_user: &str, amount: i64, source: &str, description: &str) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Internal(format!("transfer begin tx: {e}")))?;

        // Verifier le solde de l'expediteur
        let sender = sqlx::query_as::<_, WalletRow>(
            "SELECT id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at
             FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(from_user)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("transfer select sender: {e}")))?
        .ok_or_else(|| DomainError::NotFound("Portefeuille expediteur introuvable".into()))?;

        if sender.coins < amount {
            return Err(DomainError::ValidationError(format!(
                "Solde insuffisant pour ce transfert : {} coins disponibles, {} requis (manque {}).",
                sender.coins,
                amount,
                amount - sender.coins
            )));
        }

        // Debiter l'expediteur
        let sender_after = sqlx::query_scalar::<_, i64>(
            "UPDATE user_wallets SET coins = coins - $1, total_spent = total_spent + $1, updated_at = NOW()
             WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
        )
        .bind(amount)
        .bind(guild_id)
        .bind(from_user)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("transfer debit: {e}")))?;

        // Crediter le destinataire
        let receiver_after = sqlx::query_scalar::<_, i64>(
            "UPDATE user_wallets SET coins = coins + $1, total_earned = total_earned + $1, updated_at = NOW()
             WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
        )
        .bind(amount)
        .bind(guild_id)
        .bind(to_user)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("transfer credit: {e}")))?
        .ok_or_else(|| DomainError::NotFound("Portefeuille destinataire introuvable".into()))?;

        // Transactions pour l'expediteur
        sqlx::query(
            "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(from_user)
        .bind(-amount)
        .bind(sender_after)
        .bind(source)
        .bind(description)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("transfer insert tx sender: {e}")))?;

        // Transactions pour le destinataire
        sqlx::query(
            "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(to_user)
        .bind(amount)
        .bind(receiver_after)
        .bind(source)
        .bind(description)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("transfer insert tx receiver: {e}")))?;

        tx.commit().await
            .map_err(|e| DomainError::Internal(format!("transfer commit: {e}")))?;

        info!(guild_id, from_user, to_user, amount, source, "Wallet transfer");
        Ok(())
    }

    async fn pay_combat_atomic(
        &self,
        guild_id: &str,
        winner_id: &str,
        winner_amount: i64,
        loser_id: &str,
        loser_amount: i64,
        source: &str,
        description: &str,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Internal(format!("pay_combat begin tx: {e}")))?;

        // Debit perdant (si loser_amount > 0). On ne fail pas si le wallet
        // n existe pas : le combat s est deja resolu en domain, on logge
        // et on passe.
        if loser_amount > 0 {
            let loser_after = sqlx::query_scalar::<_, i64>(
                "UPDATE user_wallets SET coins = GREATEST(coins - $1, 0), total_spent = total_spent + LEAST($1, coins), updated_at = NOW() \
                 WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
            )
            .bind(loser_amount)
            .bind(guild_id)
            .bind(loser_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| DomainError::Internal(format!("pay_combat debit loser: {e}")))?;

            if let Some(balance_after) = loser_after {
                sqlx::query(
                    "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
                )
                .bind(Uuid::new_v4())
                .bind(guild_id)
                .bind(loser_id)
                .bind(-loser_amount)
                .bind(balance_after)
                .bind(source)
                .bind(description)
                .execute(&mut *tx)
                .await
                .map_err(|e| DomainError::Internal(format!("pay_combat insert tx loser: {e}")))?;
            }
        }

        // Credit gagnant (si winner_amount > 0)
        if winner_amount > 0 {
            let winner_after = sqlx::query_scalar::<_, i64>(
                "UPDATE user_wallets SET coins = coins + $1, total_earned = total_earned + $1, updated_at = NOW() \
                 WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
            )
            .bind(winner_amount)
            .bind(guild_id)
            .bind(winner_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| DomainError::Internal(format!("pay_combat credit winner: {e}")))?;

            if let Some(balance_after) = winner_after {
                sqlx::query(
                    "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
                )
                .bind(Uuid::new_v4())
                .bind(guild_id)
                .bind(winner_id)
                .bind(winner_amount)
                .bind(balance_after)
                .bind(source)
                .bind(description)
                .execute(&mut *tx)
                .await
                .map_err(|e| DomainError::Internal(format!("pay_combat insert tx winner: {e}")))?;
            }
        }

        tx.commit().await
            .map_err(|e| DomainError::Internal(format!("pay_combat commit: {e}")))?;

        info!(guild_id, winner_id, winner_amount, loser_id, loser_amount, source, "Combat payout atomic");
        Ok(())
    }

    async fn leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<Wallet>, DomainError> {
        // Phase 2 A.2 — Lit depuis la vue materialisee `mv_wallet_leaderboard`
        // refreshee toutes les 5 min par le cache-worker. Le rang est precalcule
        // donc l'ORDER BY est un index scan O(N) sur (guild_id, rank). Gain
        // typique : 100-1000x sur les hits hot. La staleness max de 5 min est
        // acceptable pour une UI de leaderboard.
        let rows = sqlx::query_as::<_, WalletRow>(
            "SELECT id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at
             FROM mv_wallet_leaderboard WHERE guild_id = $1
             ORDER BY rank
             LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("leaderboard: {e}")))?;

        Ok(rows.into_iter().map(Wallet::from).collect())
    }

    async fn get_transactions(&self, guild_id: &str, user_id: &str, limit: i64) -> Result<Vec<WalletTransaction>, DomainError> {
        let rows = sqlx::query_as::<_, WalletTransactionRow>(
            "SELECT id, guild_id, user_id, amount, balance_after, source, description, created_at
             FROM wallet_transactions WHERE guild_id = $1 AND user_id = $2
             ORDER BY created_at DESC
             LIMIT $3",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("get_transactions: {e}")))?;

        Ok(rows.into_iter().map(WalletTransaction::from).collect())
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Wallet>, DomainError> {
        let rows = sqlx::query_as::<_, WalletRow>(
            "SELECT id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at
             FROM user_wallets WHERE guild_id = $1
             ORDER BY coins DESC, updated_at DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("list_by_guild: {e}")))?;

        Ok(rows.into_iter().map(Wallet::from).collect())
    }

    async fn reset_wallet(&self, guild_id: &str, user_id: &str, new_balance: i64) -> Result<Wallet, DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Internal(format!("reset begin tx: {e}")))?;

        // Efface l'historique de transactions du joueur.
        sqlx::query("DELETE FROM wallet_transactions WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id).bind(user_id)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::Internal(format!("reset wipe tx: {e}")))?;

        // Reset le solde + total_earned/total_spent.
        let row = sqlx::query_as::<_, WalletRow>(
            r#"UPDATE user_wallets
               SET coins = $1, total_earned = $1, total_spent = 0, updated_at = NOW()
               WHERE guild_id = $2 AND user_id = $3
               RETURNING id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at"#,
        )
        .bind(new_balance)
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("reset update: {e}")))?
        .ok_or_else(|| DomainError::NotFound("Portefeuille introuvable".into()))?;

        // Log d'audit du reset en wallet_transactions.
        sqlx::query(
            "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at)
             VALUES ($1, $2, $3, $4, $5, 'reset', 'Reset admin', NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(guild_id)
        .bind(user_id)
        .bind(new_balance)
        .bind(new_balance)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("reset audit: {e}")))?;

        tx.commit().await
            .map_err(|e| DomainError::Internal(format!("reset commit: {e}")))?;

        info!(guild_id, user_id, new_balance, "Wallet reset");
        Ok(Wallet::from(row))
    }

    async fn reset_all_wallets(&self, guild_id: &str, new_balance: i64) -> Result<u64, DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Internal(format!("reset_all begin tx: {e}")))?;

        // Efface toutes les transactions de la guild.
        sqlx::query("DELETE FROM wallet_transactions WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::Internal(format!("reset_all wipe tx: {e}")))?;

        // Reset tous les wallets.
        let affected = sqlx::query(
            "UPDATE user_wallets
             SET coins = $1, total_earned = $1, total_spent = 0, updated_at = NOW()
             WHERE guild_id = $2",
        )
        .bind(new_balance)
        .bind(guild_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(format!("reset_all update: {e}")))?
        .rows_affected();

        tx.commit().await
            .map_err(|e| DomainError::Internal(format!("reset_all commit: {e}")))?;

        info!(guild_id, affected, new_balance, "Wallets bulk reset");
        Ok(affected)
    }
}

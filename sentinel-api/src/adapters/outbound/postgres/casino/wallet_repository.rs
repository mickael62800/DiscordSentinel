use crate::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use super::super::uow::as_pg;
use crate::ports::outbound::casino::wallet_repository::WalletRepository;
use sentinel_core::domain::entities::casino::wallet::Wallet;
use sentinel_core::domain::entities::casino::wallet::WalletTransaction;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::uow::DbTx;

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
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
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
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
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
    async fn get_or_create(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        starting_coins: i64,
    ) -> Result<Wallet, DomainError> {
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
        .map_err(pg_ctx("get_or_create wallet"))?;

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
        .map_err(pg_ctx("get wallet"))?;

        Ok(row.map(Wallet::from))
    }

    async fn credit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<Wallet, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_ctx("credit begin tx"))?;

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
        .map_err(pg_ctx("credit update"))?
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
        .map_err(pg_ctx("credit insert tx"))?;

        tx.commit().await.map_err(pg_ctx("credit commit"))?;

        info!(
            guild_id,
            user_id,
            amount,
            balance = wallet.coins,
            source,
            "Wallet credit"
        );
        Ok(wallet)
    }

    async fn debit(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<Wallet, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_ctx("debit begin tx"))?;

        // Verifier le solde avant debit
        let current = sqlx::query_as::<_, WalletRow>(
            "SELECT id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at
             FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_ctx("debit select"))?
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
        .map_err(pg_ctx("debit update"))?;

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
        .map_err(pg_ctx("debit insert tx"))?;

        tx.commit().await.map_err(pg_ctx("debit commit"))?;

        info!(
            guild_id,
            user_id,
            amount,
            balance = wallet.coins,
            source,
            "Wallet debit"
        );
        Ok(wallet)
    }

    async fn transfer(
        &self,
        guild_id: &str,
        from_user: &str,
        to_user: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(pg_ctx("transfer begin tx"))?;

        // Verifier le solde de l'expediteur
        let sender = sqlx::query_as::<_, WalletRow>(
            "SELECT id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at
             FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(from_user)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_ctx("transfer select sender"))?
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
        .map_err(pg_ctx("transfer debit"))?;

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
        .map_err(pg_ctx("transfer credit"))?
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
        .map_err(pg_ctx("transfer insert tx sender"))?;

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
        .map_err(pg_ctx("transfer insert tx receiver"))?;

        tx.commit().await.map_err(pg_ctx("transfer commit"))?;

        info!(
            guild_id,
            from_user, to_user, amount, source, "Wallet transfer"
        );
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
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(pg_ctx("pay_combat begin tx"))?;

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
            .map_err(pg_ctx("pay_combat debit loser"))?;

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
                .map_err(pg_ctx("pay_combat insert tx loser"))?;
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
            .map_err(pg_ctx("pay_combat credit winner"))?;

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
                .map_err(pg_ctx("pay_combat insert tx winner"))?;
            }
        }

        tx.commit().await.map_err(pg_ctx("pay_combat commit"))?;

        info!(
            guild_id,
            winner_id, winner_amount, loser_id, loser_amount, source, "Combat payout atomic"
        );
        Ok(())
    }

    async fn debit_pair_atomic(
        &self,
        guild_id: &str,
        user_a: &str,
        user_b: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<(), DomainError> {
        if amount <= 0 {
            return Ok(());
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(pg_ctx("debit_pair begin tx"))?;

        for user_id in [user_a, user_b] {
            // Clamp au solde (GREATEST) : pas d'echec si solde insuffisant,
            // coherent avec pay_combat_atomic. Ne fail pas si le wallet n'existe
            // pas (combat deja resolu).
            let after = sqlx::query_scalar::<_, i64>(
                "UPDATE user_wallets SET coins = GREATEST(coins - $1, 0), total_spent = total_spent + LEAST($1, coins), updated_at = NOW() \
                 WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
            )
            .bind(amount)
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(pg_ctx("debit_pair update"))?;

            if let Some(balance_after) = after {
                sqlx::query(
                    "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
                )
                .bind(Uuid::new_v4())
                .bind(guild_id)
                .bind(user_id)
                .bind(-amount)
                .bind(balance_after)
                .bind(source)
                .bind(description)
                .execute(&mut *tx)
                .await
                .map_err(pg_ctx("debit_pair insert tx"))?;
            }
        }

        tx.commit().await.map_err(pg_ctx("debit_pair commit"))?;

        info!(
            guild_id,
            user_a, user_b, amount, source, "Wallet debit pair atomic"
        );
        Ok(())
    }

    async fn leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<Wallet>, DomainError> {
        // Phase 2 A.2 — Lit depuis la vue materialisee `mv_wallet_leaderboard`
        // refreshee toutes les 5 min par le cache-worker. Le rang est precalcule
        // donc l'ORDER BY est un index scan O(N) sur (guild_id, rank). Gain
        // typique : 100-1000x sur les hits hot. La staleness max de 5 min est
        // acceptable pour une UI de leaderboard.
        // Filtre les membres partis (left_at NOT NULL dans guild_members).
        // NOT EXISTS plutot que JOIN : preserve les users qui n'ont jamais
        // ete syncs dans guild_members (pas filtres par accident).
        let rows = sqlx::query_as::<_, WalletRow>(
            "SELECT id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at
             FROM mv_wallet_leaderboard w WHERE guild_id = $1
             AND NOT EXISTS (SELECT 1 FROM guild_members gm
                             WHERE gm.guild_id = w.guild_id AND gm.user_id = w.user_id
                             AND gm.left_at IS NOT NULL)
             ORDER BY rank
             LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("leaderboard"))?;

        Ok(rows.into_iter().map(Wallet::from).collect())
    }

    async fn get_transactions(
        &self,
        guild_id: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<WalletTransaction>, DomainError> {
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
        .map_err(pg_ctx("get_transactions"))?;

        Ok(rows.into_iter().map(WalletTransaction::from).collect())
    }

    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Wallet>, DomainError> {
        let rows = sqlx::query_as::<_, WalletRow>(
            "SELECT id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at
             FROM user_wallets w WHERE guild_id = $1
             AND NOT EXISTS (SELECT 1 FROM guild_members gm
                             WHERE gm.guild_id = w.guild_id AND gm.user_id = w.user_id
                             AND gm.left_at IS NOT NULL)
             ORDER BY coins DESC, updated_at DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list_by_guild"))?;

        Ok(rows.into_iter().map(Wallet::from).collect())
    }

    async fn reset_wallet(
        &self,
        guild_id: &str,
        user_id: &str,
        new_balance: i64,
    ) -> Result<Wallet, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_ctx("reset begin tx"))?;

        // Efface l'historique de transactions du joueur.
        sqlx::query("DELETE FROM wallet_transactions WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(pg_ctx("reset wipe tx"))?;

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
        .map_err(pg_ctx("reset update"))?
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
        .map_err(pg_ctx("reset audit"))?;

        tx.commit().await.map_err(pg_ctx("reset commit"))?;

        info!(guild_id, user_id, new_balance, "Wallet reset");
        Ok(Wallet::from(row))
    }

    async fn reset_all_wallets(
        &self,
        guild_id: &str,
        new_balance: i64,
    ) -> Result<u64, DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(pg_ctx("reset_all begin tx"))?;

        // Efface toutes les transactions de la guild.
        sqlx::query("DELETE FROM wallet_transactions WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&mut *tx)
            .await
            .map_err(pg_ctx("reset_all wipe tx"))?;

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
        .map_err(pg_ctx("reset_all update"))?
        .rows_affected();

        tx.commit().await.map_err(pg_ctx("reset_all commit"))?;

        info!(guild_id, affected, new_balance, "Wallets bulk reset");
        Ok(affected)
    }

    async fn credit_in_tx(
        &self,
        tx: &mut dyn DbTx,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<(i64, i64), DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant credit doit etre positif".into(),
            ));
        }
        let tx = as_pg(tx);
        let previous: Option<i64> = sqlx::query_scalar(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(pg_ctx("credit_in_tx select"))?;
        let previous =
            previous.ok_or_else(|| DomainError::NotFound("Portefeuille introuvable".into()))?;
        let after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins + $1, total_earned = total_earned + $1, updated_at = NOW() \
             WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
        )
        .bind(amount).bind(guild_id).bind(user_id)
        .fetch_one(&mut **tx).await
        .map_err(pg_ctx("credit_in_tx update"))?;
        sqlx::query(
            "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
        )
        .bind(Uuid::new_v4()).bind(guild_id).bind(user_id).bind(amount).bind(after).bind(source).bind(description)
        .execute(&mut **tx).await
        .map_err(pg_ctx("insert wallet_transactions"))?;
        Ok((previous, after))
    }

    async fn debit_in_tx(
        &self,
        tx: &mut dyn DbTx,
        guild_id: &str,
        user_id: &str,
        amount: i64,
        source: &str,
        description: &str,
    ) -> Result<(i64, i64), DomainError> {
        if amount <= 0 {
            return Err(DomainError::ValidationError(
                "Montant debit doit etre positif".into(),
            ));
        }
        let tx = as_pg(tx);
        let previous: Option<i64> = sqlx::query_scalar(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(pg_ctx("debit_in_tx select"))?;
        let previous =
            previous.ok_or_else(|| DomainError::NotFound("Portefeuille introuvable".into()))?;
        if previous < amount {
            return Err(DomainError::ValidationError(format!(
                "Solde insuffisant : tu as {} coins, il en faut {} (manque {}).",
                previous,
                amount,
                amount - previous
            )));
        }
        let after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins - $1, total_spent = total_spent + $1, updated_at = NOW() \
             WHERE guild_id = $2 AND user_id = $3 RETURNING coins",
        )
        .bind(amount).bind(guild_id).bind(user_id)
        .fetch_one(&mut **tx).await
        .map_err(pg_ctx("debit_in_tx update"))?;
        sqlx::query(
            "INSERT INTO wallet_transactions (id, guild_id, user_id, amount, balance_after, source, description, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())",
        )
        .bind(Uuid::new_v4()).bind(guild_id).bind(user_id).bind(-amount).bind(after).bind(source).bind(description)
        .execute(&mut **tx).await
        .map_err(pg_ctx("insert wallet_transactions"))?;
        Ok((previous, after))
    }
}

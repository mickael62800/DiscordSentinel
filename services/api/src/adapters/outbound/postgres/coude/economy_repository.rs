use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::errors::DomainError;

use super::super::pg_err;
use crate::ports::outbound::coude::economy_repository::CoudeEconomyRepository;


pub struct PgCoudeEconomyRepository {
    pool: PgPool,
}

impl PgCoudeEconomyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}


#[async_trait]
impl CoudeEconomyRepository for PgCoudeEconomyRepository {
    // NOTE migration wallet unifie : la methode `transfer` a ete supprimee.
    // Toute la logique SQL (SELECT FOR UPDATE + UPDATE debit/credit + log
    // wallet_transactions) est centralisee dans `PgWalletRepository::transfer`
    // et orchestree par `ManageWalletService::transfer` (taunts inclus).
    // `ManageCoudeEconomyService::transfer` delegue directement au wallet UC.

    // NOTE migration wallet unifie (Migration #5) : l'ancienne methode
    // atomique `steal` a ete supprimee. Tous les call sites (ancien
    // `/voler`, daily chaos) passent maintenant par
    // `ManageWalletUseCase::transfer` + `record_steal_stats`.

    async fn record_steal_stats(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        if amount <= 0 {
            return Ok(());
        }
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        sqlx::query(
            "UPDATE coude_players SET total_lost = total_lost + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(victim_id).bind(amount)
        .execute(&mut *tx).await.map_err(pg_err)?;

        sqlx::query(
            "UPDATE coude_players SET total_stolen = total_stolen + $3, total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(thief_id).bind(amount)
        .execute(&mut *tx).await.map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }

    async fn record_steal_fail_stats(
        &self,
        guild_id: &str,
        thief_id: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        if amount <= 0 {
            return Ok(());
        }
        sqlx::query(
            "UPDATE coude_players SET total_lost = total_lost + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(thief_id).bind(amount)
        .execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }

    async fn get_coins(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        let (coins,) = row.ok_or_else(|| DomainError::NotFound("Wallet introuvable".into()))?;
        Ok(coins)
    }

    async fn record_casino_win_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Migration #5 : stats-only. La mutation wallet est faite par
        // ManageCoudeEconomyService via ManageWalletUseCase::credit.
        let result = sqlx::query(
            r#"UPDATE coude_players
               SET casino_wins = casino_wins + 1,
                   total_earned = total_earned + $3,
                   updated_at = NOW()
               WHERE guild_id = $1 AND user_id = $2"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(gain)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("Joueur introuvable".into()));
        }

        sqlx::query("INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)")
            .bind(guild_id)
            .bind(user_id)
            .bind(gain)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }

    async fn record_casino_loss_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Log la perte en negatif avant l'UPDATE pour rester coherent
        // avec le comportement legacy (log meme si le joueur n'existe pas).
        sqlx::query("INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)")
            .bind(guild_id)
            .bind(user_id)
            .bind(-lost)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;

        // Migration #5 : stats-only. La mutation wallet est faite par
        // ManageCoudeEconomyService via ManageWalletUseCase::debit.
        let result = sqlx::query(
            r#"UPDATE coude_players
               SET casino_losses = casino_losses + 1,
                   total_lost = total_lost + $3,
                   updated_at = NOW()
               WHERE guild_id = $1 AND user_id = $2"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(lost)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("Joueur introuvable".into()));
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }

    async fn record_casino_faillite_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        cleared: i64,
    ) -> Result<i64, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Migration #5 : stats-only. Le debit du solde est fait par
        // ManageCoudeEconomyService via ManageWalletUseCase::debit
        // (cleared = solde avant faillite).
        let row: (i64,) = sqlx::query_as(
            r#"UPDATE coude_players
               SET casino_losses = casino_losses + 1,
                   total_lost = total_lost + $3,
                   updated_at = NOW()
               WHERE guild_id = $1 AND user_id = $2
               RETURNING total_lost"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(cleared)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;

        if cleared > 0 {
            sqlx::query(
                "INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)",
            )
            .bind(guild_id)
            .bind(user_id)
            .bind(-cleared)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(row.0)
    }

    async fn count_casino_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM coude_cooldowns
               WHERE guild_id = $1 AND user_id = $2 AND action = 'casino'
                 AND expires_at > NOW() - INTERVAL '24 hours'"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0)
    }

    async fn sum_casino_gains_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"SELECT COALESCE(SUM(amount), 0)::bigint FROM coude_casino_log
               WHERE guild_id = $1 AND user_id = $2
                 AND amount > 0 AND created_at > NOW() - INTERVAL '24 hours'"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0)
    }

    async fn count_steal_today(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        let row: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM coude_cooldowns
               WHERE guild_id = $1 AND user_id = $2 AND action = 'voler'
                 AND expires_at > NOW() - INTERVAL '24 hours'"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.0)
    }
}

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::errors::DomainError;
use crate::ports::outbound::CoudeEconomyRepository;

pub struct PgCoudeEconomyRepository {
    pool: PgPool,
}

impl PgCoudeEconomyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.to_string())
}

#[async_trait]
impl CoudeEconomyRepository for PgCoudeEconomyRepository {
    async fn transfer(
        &self,
        guild_id: &str,
        from_id: &str,
        to_id: &str,
        amount: i64,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Phase 8 : lire le solde depuis user_wallets (wallet partage).
        let sender: Option<(i64,)> = sqlx::query_as(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(from_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

        let (sender_coins,) = sender
            .ok_or_else(|| DomainError::NotFound("Expediteur introuvable".into()))?;

        if sender_coins < amount {
            return Err(DomainError::ValidationError(format!(
                "Solde insuffisant ({} coins, {} requis)",
                sender_coins, amount
            )));
        }

        // Phase 8 : coins dans user_wallets (wallet partage).
        sqlx::query(
            "UPDATE user_wallets SET coins = coins - $3, total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(from_id)
        .bind(amount)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        let result = sqlx::query(
            "UPDATE user_wallets SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(to_id)
        .bind(amount)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("Destinataire introuvable".into()));
        }

        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }

    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<i64, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Phase 8 : lire le solde depuis user_wallets.
        let victim: Option<(i64,)> = sqlx::query_as(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(victim_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

        let (victim_coins,) = victim
            .ok_or_else(|| DomainError::NotFound("Victime introuvable".into()))?;

        // Clamp au solde réel — pas de création de coins.
        let actual_stolen = amount.min(victim_coins);
        if actual_stolen <= 0 {
            tx.commit().await.map_err(pg_err)?;
            return Ok(0);
        }

        // Debiter la victime (wallet partage).
        sqlx::query(
            "UPDATE user_wallets SET coins = coins - $3, total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(victim_id).bind(actual_stolen)
        .execute(&mut *tx).await.map_err(pg_err)?;

        // Crediter le voleur (wallet partage).
        sqlx::query(
            "UPDATE user_wallets SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(thief_id).bind(actual_stolen)
        .execute(&mut *tx).await.map_err(pg_err)?;

        // Stats coude_players (total_stolen, total_lost) — pas de mutation coins.
        sqlx::query(
            "UPDATE coude_players SET total_lost = total_lost + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(victim_id).bind(actual_stolen)
        .execute(&mut *tx).await.map_err(pg_err)?;

        sqlx::query(
            "UPDATE coude_players SET total_stolen = total_stolen + $3, total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(thief_id).bind(actual_stolen)
        .execute(&mut *tx).await.map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(actual_stolen)
    }

    async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        let result = sqlx::query(
            r#"UPDATE coude_players
               SET casino_wins = casino_wins + 1,
                   coins = coins + $3,
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

    async fn record_casino_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Log la perte en négatif avant l'UPDATE pour rester cohérent avec
        // le comportement legacy (log même si le joueur n'existe pas).
        sqlx::query("INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)")
            .bind(guild_id)
            .bind(user_id)
            .bind(-lost)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;

        let result = sqlx::query(
            r#"UPDATE coude_players
               SET casino_losses = casino_losses + 1,
                   coins = GREATEST(0, coins - $3),
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

    async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Lire le solde avant faillite pour loguer le bon montant.
        let coins_before: Option<(i64,)> = sqlx::query_as(
            "SELECT coins FROM coude_players WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

        let (coins_before,) = coins_before
            .ok_or_else(|| DomainError::NotFound("Joueur introuvable".into()))?;

        let row: (i64,) = sqlx::query_as(
            r#"UPDATE coude_players
               SET casino_losses = casino_losses + 1,
                   total_lost = total_lost + coins,
                   coins = 0,
                   updated_at = NOW()
               WHERE guild_id = $1 AND user_id = $2
               RETURNING total_lost"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;

        if coins_before > 0 {
            sqlx::query(
                "INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)",
            )
            .bind(guild_id)
            .bind(user_id)
            .bind(-coins_before)
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

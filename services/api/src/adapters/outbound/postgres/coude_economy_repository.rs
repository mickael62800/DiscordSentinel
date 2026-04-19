use async_trait::async_trait;
use sqlx::PgPool;

use crate::adapters::outbound::postgres::wallet_tx_log::log_wallet_tx;
use crate::domain::errors::DomainError;

use super::pg_err;
use crate::ports::outbound::CoudeEconomyRepository;


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

    // NOTE migration wallet unifie (/voler) : `ManageCoudeEconomyService::steal`
    // n'utilise plus cette methode repo ; il delegue a
    // `ManageWalletUseCase::transfer` + `record_steal_stats`. La methode
    // ci-dessous est conservee pour le daily chaos (non migre).
    async fn steal(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
    ) -> Result<i64, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

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

        let actual_stolen = amount.min(victim_coins);
        if actual_stolen <= 0 {
            tx.commit().await.map_err(pg_err)?;
            return Ok(0);
        }

        let victim_after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins - $3, total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id).bind(victim_id).bind(actual_stolen)
        .fetch_one(&mut *tx).await.map_err(pg_err)?;

        let thief_after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id).bind(thief_id).bind(actual_stolen)
        .fetch_one(&mut *tx).await.map_err(pg_err)?;

        let desc = format!("Vole par {}", thief_id);
        log_wallet_tx(&mut tx, guild_id, victim_id, -actual_stolen, victim_after, "coude_steal_victim", &desc).await?;
        let desc = format!("Vol sur {}", victim_id);
        log_wallet_tx(&mut tx, guild_id, thief_id, actual_stolen, thief_after, "coude_steal_thief", &desc).await?;

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

    async fn record_casino_win(
        &self,
        guild_id: &str,
        user_id: &str,
        gain: i64,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Phase 8 : credit sur user_wallets, stats sur coude_players.
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

        let balance_after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(gain)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;

        sqlx::query("INSERT INTO coude_casino_log (guild_id, user_id, amount) VALUES ($1, $2, $3)")
            .bind(guild_id)
            .bind(user_id)
            .bind(gain)
            .execute(&mut *tx)
            .await
            .map_err(pg_err)?;

        log_wallet_tx(&mut tx, guild_id, user_id, gain, balance_after, "coude_casino_win", "Blackjack gagne").await?;

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

        // Phase 8 : debit sur user_wallets, stats sur coude_players.
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

        let balance_after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = GREATEST(0, coins - $3), total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(lost)
        .fetch_one(&mut *tx)
        .await
        .map_err(pg_err)?;

        log_wallet_tx(&mut tx, guild_id, user_id, -lost, balance_after, "coude_casino_loss", "Blackjack perdu").await?;

        tx.commit().await.map_err(pg_err)?;
        Ok(())
    }

    async fn record_casino_faillite(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i64, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Phase 8 : le solde vit dans user_wallets. Lire + locker le wallet.
        let coins_before: Option<(i64,)> = sqlx::query_as(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

        let (coins_before,) = coins_before
            .ok_or_else(|| DomainError::NotFound("Joueur introuvable".into()))?;

        // Vider le wallet.
        sqlx::query(
            "UPDATE user_wallets SET coins = 0, total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(coins_before)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        if coins_before > 0 {
            log_wallet_tx(&mut tx, guild_id, user_id, -coins_before, 0, "coude_casino_faillite", "Faillite blackjack").await?;
        }

        // Maj des stats dans coude_players (casino_losses + total_lost).
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
        .bind(coins_before)
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

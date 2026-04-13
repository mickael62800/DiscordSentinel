use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::adapters::outbound::postgres::wallet_tx_log::log_wallet_tx;
use crate::domain::entities::{
    coude_title_for_level, coude_xp_for_level, CombatStat, CoudePlayer, XpProgress, COUDE_MAX_LEVEL,
};
use crate::domain::errors::DomainError;
use crate::domain::value_objects::CoudeClass;
use crate::ports::outbound::CoudePlayerRepository;

pub struct PgCoudePlayerRepository {
    pool: PgPool,
}

impl PgCoudePlayerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Toutes les colonnes de `coude_players` qu'on remonte côté domaine.
/// Phase 8 : le champ `coins` est lu depuis `user_wallets` (wallet partage)
/// au lieu de `coude_players.coins` (qui reste en DB pour compat legacy mais
/// n'est plus la source de verite). Si aucun wallet n'existe encore pour ce
/// joueur, on retourne 0 — le wallet sera cree au premier `get_or_create`.
const PLAYER_COLUMNS: &str = r#"
    cp.guild_id, cp.user_id, cp.username,
    COALESCE((SELECT w.coins FROM user_wallets w WHERE w.guild_id = cp.guild_id AND w.user_id = cp.user_id), 0) AS coins,
    cp.total_wins, cp.total_losses, cp.total_draws,
    cp.total_earned, cp.total_lost, cp.total_stolen,
    cp.cowardice_count, cp.chaos_events, cp.casino_wins, cp.casino_losses,
    cp.level, cp.xp, cp.stat_points, cp.atk, cp.def, cp.class, cp.title,
    cp.hp_current, cp.hp_max,
    cp.hp_last_regen, cp.repos_last_used, cp.class_changed_at,
    cp.season,
    cp.created_at, cp.updated_at
"#;

#[derive(sqlx::FromRow)]
struct PlayerRow {
    guild_id: String,
    user_id: String,
    username: String,
    coins: i64,
    total_wins: i32,
    total_losses: i32,
    total_draws: i32,
    total_earned: i64,
    total_lost: i64,
    total_stolen: i64,
    cowardice_count: i32,
    chaos_events: i32,
    casino_wins: i32,
    casino_losses: i32,
    level: i32,
    xp: i64,
    stat_points: i32,
    atk: i32,
    def: i32,
    class: Option<CoudeClass>,
    title: Option<String>,
    hp_current: i32,
    hp_max: i32,
    hp_last_regen: Option<DateTime<Utc>>,
    repos_last_used: Option<DateTime<Utc>>,
    class_changed_at: Option<DateTime<Utc>>,
    season: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PlayerRow> for CoudePlayer {
    fn from(r: PlayerRow) -> Self {
        Self {
            guild_id: r.guild_id,
            user_id: r.user_id,
            username: r.username,
            coins: r.coins,
            total_wins: r.total_wins,
            total_losses: r.total_losses,
            total_draws: r.total_draws,
            total_earned: r.total_earned,
            total_lost: r.total_lost,
            total_stolen: r.total_stolen,
            cowardice_count: r.cowardice_count,
            chaos_events: r.chaos_events,
            casino_wins: r.casino_wins,
            casino_losses: r.casino_losses,
            level: r.level,
            xp: r.xp,
            stat_points: r.stat_points,
            atk: r.atk,
            def: r.def,
            class: r.class,
            title: r.title,
            class_changed_at: r.class_changed_at,
            hp_current: r.hp_current,
            hp_max: r.hp_max,
            hp_last_regen: r.hp_last_regen,
            repos_last_used: r.repos_last_used,
            season: r.season,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.to_string())
}

#[async_trait]
impl CoudePlayerRepository for PgCoudePlayerRepository {
    async fn get_or_create(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CoudePlayer, DomainError> {
        // 1. Creer/mettre a jour le joueur coude
        sqlx::query(
            r#"INSERT INTO coude_players (guild_id, user_id, username)
               VALUES ($1, $2, $3)
               ON CONFLICT (guild_id, user_id)
               DO UPDATE SET username = EXCLUDED.username, updated_at = NOW()"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        // 2. Auto-creer le wallet partage si absent (starting_coins = 200).
        let starting_coins: i64 = std::env::var("WALLET_STARTING_COINS")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(200);
        sqlx::query(
            r#"INSERT INTO user_wallets (id, guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at)
               VALUES (gen_random_uuid(), $1, $2, $3, $4, $4, 0, NOW(), NOW())
               ON CONFLICT (guild_id, user_id) DO NOTHING"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(username)
        .bind(starting_coins)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        // 3. Re-fetch avec le PLAYER_COLUMNS qui lit coins depuis user_wallets
        let sql = format!(
            "SELECT {cols} FROM coude_players cp WHERE cp.guild_id = $1 AND cp.user_id = $2",
            cols = PLAYER_COLUMNS
        );
        let row: PlayerRow = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn get(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudePlayer>, DomainError> {
        let sql = format!(
            "SELECT {cols} FROM coude_players cp WHERE cp.guild_id = $1 AND cp.user_id = $2",
            cols = PLAYER_COLUMNS
        );
        let row: Option<PlayerRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn list(&self, guild_id: &str, limit: i64) -> Result<Vec<CoudePlayer>, DomainError> {
        // Phase 2 A.2 — Lit depuis la vue materialisee `mv_coude_leaderboard`
        // refreshee toutes les 5 min par le cache-worker. La MV contient toutes
        // les colonnes de coude_players + un `rank` precalcule, donc on garde
        // le meme PLAYER_COLUMNS et on remplace juste FROM + ORDER BY.
        // Staleness max 5 min — acceptable pour une UI listing.
        let sql = format!(
            r#"SELECT {cols}
               FROM mv_coude_leaderboard cp
               WHERE cp.guild_id = $1
               ORDER BY rank
               LIMIT $2"#,
            cols = PLAYER_COLUMNS
        );
        let rows: Vec<PlayerRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn random_active(
        &self,
        guild_id: &str,
        count: i64,
        min_coins: i64,
    ) -> Result<Vec<CoudePlayer>, DomainError> {
        let sql = format!(
            r#"SELECT {cols}
               FROM coude_players cp
               WHERE cp.guild_id = $1
                 AND COALESCE((SELECT w.coins FROM user_wallets w WHERE w.guild_id = cp.guild_id AND w.user_id = cp.user_id), 0) > $2
               ORDER BY RANDOM()
               LIMIT $3"#,
            cols = PLAYER_COLUMNS
        );
        let rows: Vec<PlayerRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(min_coins)
            .bind(count)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT DISTINCT guild_id FROM coude_players")
                .fetch_all(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    async fn update_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: &str,
    ) -> Result<bool, DomainError> {
        // Phase 2 A.3 — la colonne est maintenant un enum Postgres `coude_class`,
        // on cast explicitement le bind string vers l'enum.
        let result = sqlx::query(
            "UPDATE coude_players SET class = $3::coude_class, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(class)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<Option<XpProgress>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        let row: Option<(i64, i32, i32)> = sqlx::query_as(
            "SELECT xp, level, stat_points
             FROM coude_players
             WHERE guild_id = $1 AND user_id = $2
             FOR UPDATE",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;

        let Some((mut current_xp, mut current_level, mut current_stat_points)) = row else {
            return Ok(None);
        };

        let old_level = current_level;
        current_xp += amount;

        // Application déterministe du barème de niveaux du domaine.
        while current_level < COUDE_MAX_LEVEL
            && current_xp >= coude_xp_for_level(current_level + 1)
        {
            current_level += 1;
            current_stat_points += 3;
        }

        let leveled_up = current_level > old_level;
        let stat_points_gained = (current_level - old_level) * 3;
        let new_title = coude_title_for_level(current_level);

        sqlx::query(
            "UPDATE coude_players
             SET xp = $3, level = $4, stat_points = $5, title = $6, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(current_xp)
        .bind(current_level)
        .bind(current_stat_points)
        .bind(new_title)
        .execute(&mut *tx)
        .await
        .map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;

        Ok(Some(XpProgress {
            new_xp: current_xp,
            new_level: current_level,
            leveled_up,
            stat_points_gained,
        }))
    }

    async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: CombatStat,
    ) -> Result<Option<CoudePlayer>, DomainError> {
        // `stat.column()` retourne uniquement "atk" ou "def" — sûr à interpoler.
        let col = stat.column();
        let sql = format!(
            r#"UPDATE coude_players
               SET {col} = {col} + 1, stat_points = stat_points - 1, updated_at = NOW()
               WHERE guild_id = $1 AND user_id = $2 AND stat_points >= 1
               RETURNING {cols}"#,
            col = col,
            cols = PLAYER_COLUMNS
        );
        let row: Option<PlayerRow> = sqlx::query_as(&sql)
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn reset_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        cost: i64,
    ) -> Result<Option<CoudePlayer>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;

        // Verifier que le wallet a assez de coins pour payer le reset (lock).
        let wallet_coins: Option<i64> = sqlx::query_scalar(
            "SELECT coins FROM user_wallets WHERE guild_id = $1 AND user_id = $2 FOR UPDATE",
        )
        .bind(guild_id).bind(user_id)
        .fetch_optional(&mut *tx).await.map_err(pg_err)?;

        let balance = wallet_coins.unwrap_or(0);
        if balance < cost {
            tx.commit().await.map_err(pg_err)?;
            return Ok(None);
        }

        // Reset les stats dans coude_players.
        sqlx::query(
            r#"UPDATE coude_players
               SET stat_points = stat_points + atk + def,
                   atk = 0, def = 0, updated_at = NOW()
               WHERE guild_id = $1 AND user_id = $2 AND (atk > 0 OR def > 0)"#,
        )
        .bind(guild_id).bind(user_id)
        .execute(&mut *tx).await.map_err(pg_err)?;

        // Debiter le cout du reset sur le wallet partage.
        let balance_after: i64 = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins - $3, total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id).bind(user_id).bind(cost)
        .fetch_one(&mut *tx).await.map_err(pg_err)?;

        log_wallet_tx(&mut tx, guild_id, user_id, -cost, balance_after, "coude_reset_stats", "Reset des stats").await?;

        // Re-fetch le joueur avec les coins a jour.
        let sql = format!(
            "SELECT {cols} FROM coude_players cp WHERE cp.guild_id = $1 AND cp.user_id = $2",
            cols = PLAYER_COLUMNS
        );
        let row: Option<PlayerRow> = sqlx::query_as(&sql)
            .bind(guild_id).bind(user_id)
            .fetch_optional(&mut *tx).await.map_err(pg_err)?;

        tx.commit().await.map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn adjust_coins(
        &self,
        guild_id: &str,
        user_id: &str,
        delta: i64,
    ) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        let row: Option<i64> = sqlx::query_scalar(
            "UPDATE user_wallets
             SET coins = GREATEST(0, coins + $1), updated_at = NOW()
             WHERE guild_id = $2 AND user_id = $3
             RETURNING coins",
        )
        .bind(delta)
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;
        let Some(balance_after) = row else {
            tx.commit().await.map_err(pg_err)?;
            return Ok(false);
        };
        log_wallet_tx(&mut tx, guild_id, user_id, delta, balance_after, "coude_adjust", "Ajustement manuel").await?;
        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }

    async fn record_coins_earned(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        sqlx::query(
            "UPDATE coude_players SET total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(user_id).bind(amount)
        .execute(&mut *tx).await.map_err(pg_err)?;

        let row: Option<i64> = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id).bind(user_id).bind(amount)
        .fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some(balance_after) = row else {
            tx.commit().await.map_err(pg_err)?;
            return Ok(false);
        };
        log_wallet_tx(&mut tx, guild_id, user_id, amount, balance_after, "coude_earn", "Gain coude").await?;
        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }

    async fn record_coins_lost(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        sqlx::query(
            "UPDATE coude_players SET total_lost = total_lost + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(user_id).bind(amount)
        .execute(&mut *tx).await.map_err(pg_err)?;

        let row: Option<i64> = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = GREATEST(0, coins - $3), total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id).bind(user_id).bind(amount)
        .fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some(balance_after) = row else {
            tx.commit().await.map_err(pg_err)?;
            return Ok(false);
        };
        log_wallet_tx(&mut tx, guild_id, user_id, -amount, balance_after, "coude_loss", "Perte coude").await?;
        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }

    async fn record_win(
        &self,
        guild_id: &str,
        user_id: &str,
        earned: i64,
        stolen: i64,
    ) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        sqlx::query(
            r#"UPDATE coude_players
               SET total_wins = total_wins + 1,
                   total_earned = total_earned + $3,
                   total_stolen = total_stolen + $4,
                   updated_at = NOW()
               WHERE guild_id = $1 AND user_id = $2"#,
        )
        .bind(guild_id).bind(user_id).bind(earned).bind(stolen)
        .execute(&mut *tx).await.map_err(pg_err)?;

        let row: Option<i64> = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = coins + $3, total_earned = total_earned + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id).bind(user_id).bind(earned)
        .fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some(balance_after) = row else {
            tx.commit().await.map_err(pg_err)?;
            return Ok(false);
        };
        log_wallet_tx(&mut tx, guild_id, user_id, earned, balance_after, "coude_combat_win", "Combat gagne").await?;
        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }

    async fn record_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        sqlx::query(
            r#"UPDATE coude_players
               SET total_losses = total_losses + 1,
                   total_lost = total_lost + $3,
                   updated_at = NOW()
               WHERE guild_id = $1 AND user_id = $2"#,
        )
        .bind(guild_id).bind(user_id).bind(lost)
        .execute(&mut *tx).await.map_err(pg_err)?;

        let row: Option<i64> = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = GREATEST(0, coins - $3), total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id).bind(user_id).bind(lost)
        .fetch_optional(&mut *tx).await.map_err(pg_err)?;
        let Some(balance_after) = row else {
            tx.commit().await.map_err(pg_err)?;
            return Ok(false);
        };
        log_wallet_tx(&mut tx, guild_id, user_id, -lost, balance_after, "coude_combat_loss", "Combat perdu").await?;
        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }

    async fn record_draw(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<bool, DomainError> {
        let mut tx = self.pool.begin().await.map_err(pg_err)?;
        sqlx::query(
            "UPDATE coude_players SET total_draws = total_draws + 1, total_lost = total_lost + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id).bind(user_id).bind(lost)
        .execute(&mut *tx).await.map_err(pg_err)?;

        let row: Option<i64> = sqlx::query_scalar(
            "UPDATE user_wallets SET coins = GREATEST(0, coins - $3), total_spent = total_spent + $3, updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2
             RETURNING coins",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(lost)
        .fetch_optional(&mut *tx)
        .await
        .map_err(pg_err)?;
        let Some(balance_after) = row else {
            tx.commit().await.map_err(pg_err)?;
            return Ok(false);
        };
        log_wallet_tx(&mut tx, guild_id, user_id, -lost, balance_after, "coude_combat_draw", "Combat egalite").await?;
        tx.commit().await.map_err(pg_err)?;
        Ok(true)
    }

    async fn increment_cowardice(
        &self,
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
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(|r| r.0))
    }

    async fn increment_chaos(
        &self,
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
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn update_hp(
        &self,
        guild_id: &str,
        user_id: &str,
        hp_current: i32,
        hp_max: i32,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE coude_players
             SET hp_current = $3, hp_max = $4, hp_last_regen = NOW(), updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(hp_current)
        .bind(hp_max)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn full_heal(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE coude_players
             SET hp_current = hp_max,
                 repos_last_used = NOW(),
                 hp_last_regen = NOW(),
                 updated_at = NOW()
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}

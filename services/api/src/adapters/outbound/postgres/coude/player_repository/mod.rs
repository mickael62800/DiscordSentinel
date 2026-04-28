use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;

use crate::domain::entities::coude::player::CombatStat;
use crate::domain::entities::coude::player::CoudePlayer;
use crate::domain::entities::coude::player::XpProgress;
use crate::domain::errors::DomainError;

use crate::adapters::outbound::postgres::pg_err;
use crate::domain::enums::coude::coude_class::CoudeClass;
use crate::ports::outbound::coude::player_repository::CoudePlayerRepository;

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


mod coins;
mod combat_stats;
mod hp;
mod progression;
mod read;
mod streaks;

#[async_trait]
impl CoudePlayerRepository for PgCoudePlayerRepository {
    async fn get_or_create(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<CoudePlayer, DomainError> {
        read::get_or_create(self, guild_id, user_id, username).await
    }

    async fn get(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoudePlayer>, DomainError> {
        read::get(self, guild_id, user_id).await
    }

    async fn list(&self, guild_id: &str, limit: i64) -> Result<Vec<CoudePlayer>, DomainError> {
        read::list(self, guild_id, limit).await
    }

    async fn random_active(
        &self,
        guild_id: &str,
        count: i64,
        min_coins: i64,
    ) -> Result<Vec<CoudePlayer>, DomainError> {
        read::random_active(self, guild_id, count, min_coins).await
    }

    async fn list_guild_ids(&self) -> Result<Vec<String>, DomainError> {
        read::list_guild_ids(self).await
    }

    async fn update_class(
        &self,
        guild_id: &str,
        user_id: &str,
        class: &str,
    ) -> Result<bool, DomainError> {
        progression::update_class(self, guild_id, user_id, class).await
    }

    async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<Option<XpProgress>, DomainError> {
        progression::add_xp(self, guild_id, user_id, amount).await
    }

    async fn spend_stat_point(
        &self,
        guild_id: &str,
        user_id: &str,
        stat: CombatStat,
    ) -> Result<Option<CoudePlayer>, DomainError> {
        progression::spend_stat_point(self, guild_id, user_id, stat).await
    }

    async fn reset_stats(
        &self,
        guild_id: &str,
        user_id: &str,
        cost: i64,
    ) -> Result<Option<CoudePlayer>, DomainError> {
        progression::reset_stats(self, guild_id, user_id, cost).await
    }

    async fn record_coins_earned(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<bool, DomainError> {
        coins::record_coins_earned(self, guild_id, user_id, amount).await
    }

    async fn record_coins_lost(
        &self,
        guild_id: &str,
        user_id: &str,
        amount: i64,
    ) -> Result<bool, DomainError> {
        coins::record_coins_lost(self, guild_id, user_id, amount).await
    }

    async fn record_win(
        &self,
        guild_id: &str,
        user_id: &str,
        earned: i64,
        stolen: i64,
    ) -> Result<bool, DomainError> {
        combat_stats::record_win(self, guild_id, user_id, earned, stolen).await
    }

    async fn record_loss(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<bool, DomainError> {
        combat_stats::record_loss(self, guild_id, user_id, lost).await
    }

    async fn record_draw(
        &self,
        guild_id: &str,
        user_id: &str,
        lost: i64,
    ) -> Result<bool, DomainError> {
        combat_stats::record_draw(self, guild_id, user_id, lost).await
    }

    async fn increment_cowardice(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError> {
        combat_stats::increment_cowardice(self, guild_id, user_id).await
    }

    // ── Streaks (Phase 9 Part D) ──

    async fn touch_win_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError> {
        streaks::touch_win_streak(self, guild_id, user_id).await
    }

    async fn touch_loss_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError> {
        streaks::touch_loss_streak(self, guild_id, user_id).await
    }

    async fn reset_combat_streaks(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        streaks::reset_combat_streaks(self, guild_id, user_id).await
    }

    async fn get_combat_streaks(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<(i32, i32)>, DomainError> {
        let row: Option<(i32, i32)> = sqlx::query_as(
            r#"SELECT current_win_streak, current_loss_streak
               FROM coude_players
               WHERE guild_id = $1 AND user_id = $2"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::adapters::outbound::postgres::pg_err)?;
        Ok(row)
    }

    async fn increment_friendly_stat(
        &self,
        guild_id: &str,
        user_id: &str,
        won: bool,
    ) -> Result<(), DomainError> {
        // Deux requetes statiques plutot qu un format!() avec interpolation
        // de nom de colonne — eviter tout risque d injection si la fonction
        // est refactoree ulterieurement avec un input non controle.
        let result = if won {
            sqlx::query(
                r#"UPDATE coude_players
                   SET friendly_wins = friendly_wins + 1, updated_at = NOW()
                   WHERE guild_id = $1 AND user_id = $2"#,
            )
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"UPDATE coude_players
                   SET friendly_losses = friendly_losses + 1, updated_at = NOW()
                   WHERE guild_id = $1 AND user_id = $2"#,
            )
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
        };
        result.map_err(crate::adapters::outbound::postgres::pg_err)?;
        Ok(())
    }

    async fn get_prestige_count(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError> {
        let row: Option<(i32,)> = sqlx::query_as(
            r#"SELECT prestige_count
               FROM coude_players
               WHERE guild_id = $1 AND user_id = $2"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(crate::adapters::outbound::postgres::pg_err)?;
        Ok(row.map(|(c,)| c))
    }

    async fn touch_steal_victim_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError> {
        streaks::touch_steal_victim_streak(self, guild_id, user_id).await
    }

    async fn reset_steal_victim_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        streaks::reset_steal_victim_streak(self, guild_id, user_id).await
    }

    async fn touch_bj_win_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError> {
        streaks::touch_bj_win_streak(self, guild_id, user_id).await
    }

    async fn touch_bj_bust_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<i32>, DomainError> {
        streaks::touch_bj_bust_streak(self, guild_id, user_id).await
    }

    async fn reset_bj_bust_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        streaks::reset_bj_bust_streak(self, guild_id, user_id).await
    }

    async fn increment_chaos(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<bool, DomainError> {
        combat_stats::increment_chaos(self, guild_id, user_id).await
    }

    async fn update_hp(
        &self,
        guild_id: &str,
        user_id: &str,
        hp_current: i32,
        hp_max: i32,
    ) -> Result<(), DomainError> {
        hp::update_hp(self, guild_id, user_id, hp_current, hp_max).await
    }

    async fn full_heal(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        hp::full_heal(self, guild_id, user_id).await
    }

    async fn regen_hp_tick(
        &self,
        rate_0_25: f64,
        rate_25_50: f64,
        rate_50_75: f64,
        rate_75_100: f64,
    ) -> Result<u64, DomainError> {
        hp::regen_hp_tick(self, rate_0_25, rate_25_50, rate_50_75, rate_75_100).await
    }
}

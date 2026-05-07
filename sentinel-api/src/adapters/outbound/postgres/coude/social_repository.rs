use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::social::Season;
use sentinel_core::domain::entities::coude::social::Event;
use sentinel_core::domain::entities::coude::social::LeaderboardEntry;
use sentinel_core::domain::entities::coude::social::LeaderboardCategory;
use sentinel_core::domain::entities::coude::social::NewDailyChaos;
use sentinel_core::domain::errors::DomainError;

use super::super::pg_err;
use crate::ports::outbound::coude::social_repository::SocialRepository;

pub struct PgSocialRepository {
    pool: PgPool,
}

impl PgSocialRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}


#[derive(sqlx::FromRow)]
struct LeaderboardRow {
    user_id: String,
    username: String,
    value: i64,
}

impl From<LeaderboardRow> for LeaderboardEntry {
    fn from(r: LeaderboardRow) -> Self {
        Self {
            user_id: r.user_id.into(),
            username: r.username,
            value: r.value,
        }
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    guild_id: String,
    event_type: String,
    active: bool,
    expires_at: DateTime<Utc>,
    #[sqlx(rename = "started_at")]
    created_at: DateTime<Utc>,
}

impl From<EventRow> for Event {
    fn from(r: EventRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            event_type: r.event_type,
            active: r.active,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl SocialRepository for PgSocialRepository {
    // ── Cooldowns ──

    async fn get_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<DateTime<Utc>>, DomainError> {
        let row: Option<(DateTime<Utc>,)> = sqlx::query_as(
            r#"SELECT expires_at FROM coude_cooldowns
               WHERE guild_id = $1 AND user_id = $2 AND action = $3 AND expires_at > NOW()"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(action)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(|r| r.0))
    }

    async fn set_cooldown(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        duration_secs: i64,
    ) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_cooldowns (guild_id, user_id, action, expires_at)
               VALUES ($1, $2, $3, NOW() + make_interval(secs => $4::double precision))
               ON CONFLICT (guild_id, user_id, action)
               DO UPDATE SET expires_at = NOW() + make_interval(secs => $4::double precision)"#,
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(action)
        .bind(duration_secs as f64)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    // ── Leaderboard ──

    async fn leaderboard(
        &self,
        guild_id: &str,
        category: LeaderboardCategory,
        limit: i64,
    ) -> Result<Vec<LeaderboardEntry>, DomainError> {
        // Un SELECT par catégorie — l'enum garantit que rien d'autre ne passe
        // (pas d'interpolation de strings arbitraires).
        let sql = match category {
            LeaderboardCategory::Richest => {
                // Depuis la migration 080, `coude_players.coins` est une
                // colonne legacy frozen. Le solde reel vit dans
                // `user_wallets` (wallet partage entre jeux). On JOIN pour
                // ne remonter que les joueurs Coude actifs, triees par
                // leur solde wallet.
                "SELECT cp.user_id, cp.username, w.coins AS value \
                 FROM coude_players cp \
                 INNER JOIN user_wallets w \
                   ON w.guild_id = cp.guild_id AND w.user_id = cp.user_id \
                 WHERE cp.guild_id = $1 \
                 ORDER BY w.coins DESC \
                 LIMIT $2"
            }
            LeaderboardCategory::Thieves => {
                "SELECT user_id, username, total_stolen AS value FROM coude_players \
                 WHERE guild_id = $1 ORDER BY total_stolen DESC LIMIT $2"
            }
            LeaderboardCategory::Cowards => {
                "SELECT user_id, username, cowardice_count::BIGINT AS value FROM coude_players \
                 WHERE guild_id = $1 ORDER BY cowardice_count DESC LIMIT $2"
            }
            LeaderboardCategory::Chaos => {
                "SELECT user_id, username, chaos_events::BIGINT AS value FROM coude_players \
                 WHERE guild_id = $1 ORDER BY chaos_events DESC LIMIT $2"
            }
            LeaderboardCategory::Level => {
                "SELECT user_id, username, level::BIGINT AS value FROM coude_players \
                 WHERE guild_id = $1 ORDER BY level DESC, xp DESC LIMIT $2"
            }
        };

        let rows: Vec<LeaderboardRow> = sqlx::query_as(sql)
            .bind(guild_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    // ── Événements ──

    async fn list_active_events(&self, guild_id: &str) -> Result<Vec<Event>, DomainError> {
        let rows: Vec<EventRow> = sqlx::query_as(
            r#"SELECT id, guild_id, event_type, active, expires_at, started_at
               FROM coude_events
               WHERE guild_id = $1 AND active = TRUE AND expires_at > NOW()"#,
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    // ── Daily chaos ──

    async fn log_daily_chaos(&self, chaos: NewDailyChaos) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO coude_daily_chaos
                 (guild_id, loser_id, loser_name, winner_id, winner_name, amount)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
        )
        .bind(chaos.guild_id.as_str())
        .bind(&chaos.loser_id)
        .bind(&chaos.loser_name)
        .bind(&chaos.winner_id)
        .bind(&chaos.winner_name)
        .bind(chaos.amount)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn count_daily_chaos_today(&self, guild_id: &str) -> Result<i64, DomainError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM coude_daily_chaos WHERE guild_id = $1 AND created_at >= CURRENT_DATE",
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(count.0)
    }

    // ── Saison ──

    async fn get_or_bootstrap_current_season(
        &self,
        guild_id: &str,
    ) -> Result<Season, DomainError> {
        // 1. Tenter de récupérer la saison active (ended_at IS NULL).
        let existing: Option<(i32, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT season_number, started_at
               FROM coude_seasons
               WHERE guild_id = $1 AND ended_at IS NULL
               ORDER BY season_number DESC
               LIMIT 1"#,
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        let (season_number, started_at) = if let Some(row) = existing {
            row
        } else {
            // 2. Bootstrap : créer la saison suivante.
            let row: (i32, DateTime<Utc>) = sqlx::query_as(
                r#"INSERT INTO coude_seasons (guild_id, season_number, started_at)
                   VALUES (
                       $1,
                       COALESCE(
                           (SELECT MAX(season_number) FROM coude_seasons WHERE guild_id = $1),
                           0
                       ) + 1,
                       NOW()
                   )
                   RETURNING season_number, started_at"#,
            )
            .bind(guild_id)
            .fetch_one(&self.pool)
            .await
            .map_err(pg_err)?;
            row
        };

        let ends_at = started_at + chrono::Duration::days(90);
        let days_remaining = (ends_at - Utc::now()).num_days().max(0);

        Ok(Season {
            season_number,
            started_at,
            ends_at,
            days_remaining,
        })
    }
}

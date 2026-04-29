use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::entities::community::conduct::ConductConfig;
use crate::domain::entities::community::conduct::ConductPointsLog;
use crate::domain::entities::community::conduct::UserConductPoints;
use crate::domain::errors::DomainError;
use crate::ports::outbound::community::conduct_repository::ConductRepository;
use crate::domain::entities::system::discord_ids::UserId;

pub struct PgConductRepository {
    pool: PgPool,
}

impl PgConductRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ConfigRow {
    guild_id: String,
    max_points: i32,
    regen_amount: i32,
    regen_interval: String,
    penalty_warn: i32,
    penalty_delete: i32,
    penalty_mute: i32,
    penalty_ban: i32,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<ConfigRow> for ConductConfig {
    fn from(row: ConfigRow) -> Self {
        Self {
            guild_id: row.guild_id,
            max_points: row.max_points,
            regen_amount: row.regen_amount,
            regen_interval: row.regen_interval,
            penalty_warn: row.penalty_warn,
            penalty_delete: row.penalty_delete,
            penalty_mute: row.penalty_mute,
            penalty_ban: row.penalty_ban,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PointsRow {
    id: uuid::Uuid,
    guild_id: String,
    user_id: UserId,
    username: String,
    points: i32,
    last_regen_at: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<PointsRow> for UserConductPoints {
    fn from(row: PointsRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id,
            user_id: row.user_id,
            username: row.username,
            points: row.points,
            last_regen_at: row.last_regen_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct LogRow {
    id: uuid::Uuid,
    guild_id: String,
    user_id: UserId,
    delta: i32,
    reason: String,
    points_before: i32,
    points_after: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<LogRow> for ConductPointsLog {
    fn from(row: LogRow) -> Self {
        Self {
            id: row.id,
            guild_id: row.guild_id,
            user_id: row.user_id,
            delta: row.delta,
            reason: row.reason,
            points_before: row.points_before,
            points_after: row.points_after,
            created_at: row.created_at,
        }
    }
}

#[async_trait]
impl ConductRepository for PgConductRepository {
    // ── Config ──

    async fn get_config(&self, guild_id: &str) -> Result<Option<ConductConfig>, DomainError> {
        let row = sqlx::query_as::<_, ConfigRow>(
            "SELECT * FROM conduct_config WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(ConductConfig::from))
    }

    async fn save_config(&self, config: &ConductConfig) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO conduct_config (guild_id, max_points, regen_amount, regen_interval, penalty_warn, penalty_delete, penalty_mute, penalty_ban, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (guild_id) DO UPDATE SET
                max_points = EXCLUDED.max_points,
                regen_amount = EXCLUDED.regen_amount,
                regen_interval = EXCLUDED.regen_interval,
                penalty_warn = EXCLUDED.penalty_warn,
                penalty_delete = EXCLUDED.penalty_delete,
                penalty_mute = EXCLUDED.penalty_mute,
                penalty_ban = EXCLUDED.penalty_ban,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(&config.guild_id)
        .bind(config.max_points)
        .bind(config.regen_amount)
        .bind(&config.regen_interval)
        .bind(config.penalty_warn)
        .bind(config.penalty_delete)
        .bind(config.penalty_mute)
        .bind(config.penalty_ban)
        .bind(config.created_at)
        .bind(config.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    // ── Points ──

    async fn get_points(&self, guild_id: &str, user_id: &str) -> Result<Option<UserConductPoints>, DomainError> {
        let row = sqlx::query_as::<_, PointsRow>(
            "SELECT * FROM user_conduct_points WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(UserConductPoints::from))
    }

    async fn save_points(&self, points: &UserConductPoints) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO user_conduct_points (id, guild_id, user_id, username, points, last_regen_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = EXCLUDED.username,
                points = EXCLUDED.points,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(points.id)
        .bind(&points.guild_id)
        .bind(&points.user_id)
        .bind(&points.username)
        .bind(points.points)
        .bind(points.last_regen_at)
        .bind(points.created_at)
        .bind(points.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn update_points(&self, guild_id: &str, user_id: &str, new_points: i32) -> Result<(), DomainError> {
        sqlx::query("UPDATE user_conduct_points SET points = $1, updated_at = NOW() WHERE guild_id = $2 AND user_id = $3")
            .bind(new_points)
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_leaderboard(&self, guild_id: &str, limit: i64) -> Result<Vec<UserConductPoints>, DomainError> {
        let rows = sqlx::query_as::<_, PointsRow>(
            "SELECT ucp.id, ucp.guild_id, ucp.user_id,
                    COALESCE(NULLIF(ucp.username, ''), gm.display_name, gm.username, ucp.user_id) AS username,
                    ucp.points, ucp.last_regen_at, ucp.created_at, ucp.updated_at
             FROM user_conduct_points ucp
             LEFT JOIN guild_members gm ON gm.guild_id = ucp.guild_id AND gm.user_id = ucp.user_id
             WHERE ucp.guild_id = $1 AND (gm.is_bot IS NULL OR gm.is_bot = FALSE)
             ORDER BY ucp.points DESC, ucp.updated_at DESC LIMIT $2",
        )
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(UserConductPoints::from).collect())
    }

    async fn find_users_needing_regen(&self, interval: &str) -> Result<Vec<UserConductPoints>, DomainError> {
        // SECURITE : interval_expr est strictement controle par le match (jamais d'input utilisateur dans le SQL)
        let interval_expr = match interval {
            "weekly" => "7 days",
            "monthly" => "30 days",
            other => {
                tracing::warn!(interval = %other, "Intervalle de regen inconnu, fallback 7 days");
                "7 days"
            }
        };

        let query = format!(
            "SELECT ucp.* FROM user_conduct_points ucp \
             JOIN conduct_config cc ON cc.guild_id = ucp.guild_id \
             WHERE cc.regen_interval = $1 \
             AND ucp.last_regen_at + INTERVAL '{}' <= NOW()",
            interval_expr
        );

        let rows = sqlx::query_as::<_, PointsRow>(&query)
            .bind(interval)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(UserConductPoints::from).collect())
    }

    async fn update_regen_timestamp(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE user_conduct_points SET last_regen_at = NOW() WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn delete_points(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM user_conduct_points WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    // ── Log ──

    async fn save_log(&self, log: &ConductPointsLog) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO conduct_points_log (id, guild_id, user_id, delta, reason, points_before, points_after, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(log.id)
        .bind(&log.guild_id)
        .bind(&log.user_id)
        .bind(log.delta)
        .bind(&log.reason)
        .bind(log.points_before)
        .bind(log.points_after)
        .bind(log.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn get_log(&self, guild_id: &str, user_id: &str, limit: i64) -> Result<Vec<ConductPointsLog>, DomainError> {
        let rows = sqlx::query_as::<_, LogRow>(
            "SELECT * FROM conduct_points_log WHERE guild_id = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT $3",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(ConductPointsLog::from).collect())
    }

    async fn find_zero_points_users_without_ban_proposal(
        &self,
        reason_prefix: &str,
    ) -> Result<Vec<UserConductPoints>, DomainError> {
        let pattern = format!("{reason_prefix}%");
        let rows = sqlx::query_as::<_, PointsRow>(
            "SELECT ucp.* FROM user_conduct_points ucp \
             WHERE ucp.points <= 0 \
               AND NOT EXISTS ( \
                   SELECT 1 FROM infractions i \
                   WHERE i.guild_id = ucp.guild_id \
                     AND i.user_id = ucp.user_id \
                     AND i.action = 'ban' \
                     AND i.reason LIKE $1 \
               )",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(UserConductPoints::from).collect())
    }
}

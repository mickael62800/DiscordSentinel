use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::entities::WatchedUser;
use crate::domain::errors::DomainError;
use crate::ports::outbound::WatchedUserRepository;

pub struct PgWatchedUserRepository {
    pool: PgPool,
}

impl PgWatchedUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct WatchedUserRow {
    user_id: String,
    username: String,
    guild_id: String,
    guild_name: String,
    total_warns: i64,
    total_mutes: i64,
    total_bans: i64,
    conduct_points: Option<i32>,
    max_conduct_points: Option<i32>,
    last_incident_at: Option<chrono::DateTime<chrono::Utc>>,
    security_events_count: i64,
    first_seen_at: chrono::DateTime<chrono::Utc>,
}

impl From<WatchedUserRow> for WatchedUser {
    fn from(row: WatchedUserRow) -> Self {
        let total = row.total_warns + row.total_mutes + row.total_bans;
        let risk_level = if row.total_bans > 0 || total >= 5 {
            "critical".to_string()
        } else if row.total_mutes > 0 || total >= 3 {
            "high".to_string()
        } else if total >= 2 {
            "medium".to_string()
        } else {
            "low".to_string()
        };

        Self {
            user_id: row.user_id,
            username: row.username,
            guild_id: row.guild_id,
            guild_name: row.guild_name,
            risk_level,
            total_warns: row.total_warns,
            total_mutes: row.total_mutes,
            total_bans: row.total_bans,
            conduct_points: row.conduct_points,
            max_conduct_points: row.max_conduct_points,
            last_incident_at: row.last_incident_at,
            security_events_count: row.security_events_count,
            first_seen_at: row.first_seen_at,
        }
    }
}

#[async_trait]
impl WatchedUserRepository for PgWatchedUserRepository {
    async fn find_watched_users(
        &self,
        guild_id: Option<&str>,
    ) -> Result<Vec<WatchedUser>, DomainError> {
        let query = r#"
            WITH user_infractions AS (
                SELECT
                    i.guild_id,
                    i.user_id,
                    i.username,
                    COUNT(*) FILTER (WHERE i.action = 'warn') AS total_warns,
                    COUNT(*) FILTER (WHERE i.action = 'mute') AS total_mutes,
                    COUNT(*) FILTER (WHERE i.action = 'ban') AS total_bans,
                    MAX(i.created_at) AS last_incident_at,
                    MIN(i.created_at) AS first_seen_at
                FROM infractions i
                WHERE ($1::text IS NULL OR i.guild_id = $1)
                GROUP BY i.guild_id, i.user_id, i.username
            ),
            user_security AS (
                SELECT
                    se.guild_id,
                    u.user_id,
                    COUNT(*) AS security_events_count
                FROM security_events se,
                     jsonb_array_elements_text(se.user_ids) AS u(user_id)
                WHERE ($1::text IS NULL OR se.guild_id = $1)
                GROUP BY se.guild_id, u.user_id
            )
            SELECT
                ui.user_id,
                ui.username,
                ui.guild_id,
                COALESCE(g.name, ui.guild_id) AS guild_name,
                ui.total_warns,
                ui.total_mutes,
                ui.total_bans,
                ucp.points AS conduct_points,
                cc.max_points AS max_conduct_points,
                ui.last_incident_at,
                COALESCE(us.security_events_count, 0) AS security_events_count,
                ui.first_seen_at
            FROM user_infractions ui
            LEFT JOIN guilds g ON g.guild_id = ui.guild_id
            LEFT JOIN user_conduct_points ucp ON ucp.guild_id = ui.guild_id AND ucp.user_id = ui.user_id
            LEFT JOIN conduct_config cc ON cc.guild_id = ui.guild_id
            LEFT JOIN user_security us ON us.guild_id = ui.guild_id AND us.user_id = ui.user_id
            ORDER BY (ui.total_warns + ui.total_mutes + ui.total_bans) DESC, ui.last_incident_at DESC
            LIMIT 200
        "#;

        let rows = sqlx::query_as::<_, WatchedUserRow>(query)
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(WatchedUser::from).collect())
    }
}

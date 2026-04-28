use async_trait::async_trait;
use sqlx::PgPool;

use crate::ports::outbound::audit::modstats_repository::ModeratorStat;
use crate::ports::outbound::audit::modstats_repository::ModstatsRepository;
use super::pg_err;

pub struct PgModstatsRepository { pool: PgPool }

impl PgModstatsRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[derive(sqlx::FromRow)]
struct Row {
    moderator_id: String,
    moderator_name: String,
    action_count: i64,
}

#[async_trait]
impl ModstatsRepository for PgModstatsRepository {
    async fn top_moderators(
        &self, guild_id: &str, days: i32, limit: i64,
    ) -> Result<Vec<ModeratorStat>, crate::domain::errors::DomainError> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT moderator_id, moderator_name, COUNT(*) AS action_count \
             FROM moderation_actions \
             WHERE guild_id = $1 AND created_at >= NOW() - make_interval(days => $2) \
             GROUP BY moderator_id, moderator_name \
             ORDER BY action_count DESC \
             LIMIT $3",
        )
        .bind(guild_id).bind(days).bind(limit)
        .fetch_all(&self.pool).await.map_err(pg_err)?;

        Ok(rows.into_iter().map(|r| ModeratorStat {
            moderator_id: r.moderator_id,
            moderator_name: r.moderator_name,
            action_count: r.action_count,
        }).collect())
    }
}

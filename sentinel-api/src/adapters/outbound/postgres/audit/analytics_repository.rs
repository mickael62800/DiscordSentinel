use async_trait::async_trait;
use sqlx::PgPool;

use sentinel_core::domain::entities::system::analytics::*;
use sentinel_core::domain::errors::DomainError;
use crate::ports::outbound::audit::analytics_repository::AnalyticsRepository;

pub struct PgAnalyticsRepository {
    pool: PgPool,
}

impl PgAnalyticsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AnalyticsRepository for PgAnalyticsRepository {
    async fn get_heatmap(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<HourlyActivity>, DomainError> {
        let rows: Vec<(i16, Option<f64>, Option<i64>, Option<i64>)> = if let Some(gid) = guild_id {
            sqlx::query_as(
                "SELECT hour, EXTRACT(ISODOW FROM day)::float8 - 1 AS day_of_week, \
                 SUM(messages)::bigint AS messages, SUM(infractions)::bigint AS infractions \
                 FROM hourly_activity WHERE guild_id = $1 AND day >= CURRENT_DATE - $2::integer \
                 GROUP BY hour, EXTRACT(ISODOW FROM day) ORDER BY day_of_week, hour",
            )
            .bind(gid)
            .bind(days)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                "SELECT hour, EXTRACT(ISODOW FROM day)::float8 - 1 AS day_of_week, \
                 SUM(messages)::bigint AS messages, SUM(infractions)::bigint AS infractions \
                 FROM hourly_activity WHERE day >= CURRENT_DATE - $1::integer \
                 GROUP BY hour, EXTRACT(ISODOW FROM day) ORDER BY day_of_week, hour",
            )
            .bind(days)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(hour, dow, msgs, infr)| HourlyActivity {
                hour,
                day_of_week: dow.unwrap_or(0.0) as i16,
                messages: msgs.unwrap_or(0),
                infractions: infr.unwrap_or(0) as i32,
            })
            .collect())
    }

    async fn get_action_distribution(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<ActionDistribution>, DomainError> {
        let rows: Vec<(String, Option<i64>)> = if let Some(gid) = guild_id {
            sqlx::query_as(
                "SELECT action, COUNT(*)::bigint AS count FROM infractions \
                 WHERE guild_id = $1 AND created_at >= NOW() - make_interval(days => $2) \
                 AND action != 'none' GROUP BY action ORDER BY count DESC",
            )
            .bind(gid)
            .bind(days)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                "SELECT action, COUNT(*)::bigint AS count FROM infractions \
                 WHERE created_at >= NOW() - make_interval(days => $1) \
                 AND action != 'none' GROUP BY action ORDER BY count DESC",
            )
            .bind(days)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        let total: i64 = rows.iter().map(|(_, c)| c.unwrap_or(0)).sum();

        Ok(rows
            .into_iter()
            .map(|(action, count)| {
                let c = count.unwrap_or(0);
                ActionDistribution {
                    action,
                    count: c,
                    percentage: if total > 0 {
                        (c as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    },
                }
            })
            .collect())
    }

    async fn get_top_infractors(
        &self,
        guild_id: Option<&str>,
        days: i32,
        limit: i64,
    ) -> Result<Vec<TopInfractor>, DomainError> {
        let rows: Vec<(String, String, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>)> =
            if let Some(gid) = guild_id {
                sqlx::query_as(
                    "SELECT user_id, username, COUNT(*)::bigint AS total, \
                     COUNT(*) FILTER (WHERE action = 'warn')::bigint AS warns, \
                     COUNT(*) FILTER (WHERE action = 'delete')::bigint AS deletes, \
                     COUNT(*) FILTER (WHERE action = 'mute')::bigint AS mutes, \
                     COUNT(*) FILTER (WHERE action = 'ban')::bigint AS bans \
                     FROM infractions WHERE guild_id = $1 AND created_at >= NOW() - make_interval(days => $2) \
                     AND action != 'none' GROUP BY user_id, username ORDER BY total DESC LIMIT $3",
                )
                .bind(gid)
                .bind(days)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
            } else {
                sqlx::query_as(
                    "SELECT user_id, username, COUNT(*)::bigint AS total, \
                     COUNT(*) FILTER (WHERE action = 'warn')::bigint AS warns, \
                     COUNT(*) FILTER (WHERE action = 'delete')::bigint AS deletes, \
                     COUNT(*) FILTER (WHERE action = 'mute')::bigint AS mutes, \
                     COUNT(*) FILTER (WHERE action = 'ban')::bigint AS bans \
                     FROM infractions WHERE created_at >= NOW() - make_interval(days => $1) \
                     AND action != 'none' GROUP BY user_id, username ORDER BY total DESC LIMIT $2",
                )
                .bind(days)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
            }
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(uid, uname, total, w, d, m, b)| TopInfractor {
                user_id: uid.into(),
                username: uname,
                total_infractions: total.unwrap_or(0),
                warns: w.unwrap_or(0),
                deletes: d.unwrap_or(0),
                mutes: m.unwrap_or(0),
                bans: b.unwrap_or(0),
            })
            .collect())
    }

    async fn get_moderation_trend(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<ModerationTrend>, DomainError> {
        let rows: Vec<(Option<chrono::NaiveDate>, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>)> =
            if let Some(gid) = guild_id {
                sqlx::query_as(
                    "SELECT created_at::date AS day, COUNT(*)::bigint AS total, \
                     COUNT(*) FILTER (WHERE action = 'warn')::bigint AS warns, \
                     COUNT(*) FILTER (WHERE action = 'delete')::bigint AS deletes, \
                     COUNT(*) FILTER (WHERE action = 'mute')::bigint AS mutes, \
                     COUNT(*) FILTER (WHERE action = 'ban')::bigint AS bans \
                     FROM infractions WHERE guild_id = $1 AND created_at >= NOW() - make_interval(days => $2) \
                     AND action != 'none' GROUP BY created_at::date ORDER BY day",
                )
                .bind(gid)
                .bind(days)
                .fetch_all(&self.pool)
                .await
            } else {
                sqlx::query_as(
                    "SELECT created_at::date AS day, COUNT(*)::bigint AS total, \
                     COUNT(*) FILTER (WHERE action = 'warn')::bigint AS warns, \
                     COUNT(*) FILTER (WHERE action = 'delete')::bigint AS deletes, \
                     COUNT(*) FILTER (WHERE action = 'mute')::bigint AS mutes, \
                     COUNT(*) FILTER (WHERE action = 'ban')::bigint AS bans \
                     FROM infractions WHERE created_at >= NOW() - make_interval(days => $1) \
                     AND action != 'none' GROUP BY created_at::date ORDER BY day",
                )
                .bind(days)
                .fetch_all(&self.pool)
                .await
            }
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(day, total, w, d, m, b)| ModerationTrend {
                day: day.unwrap_or_default(),
                total: total.unwrap_or(0),
                warns: w.unwrap_or(0),
                deletes: d.unwrap_or(0),
                mutes: m.unwrap_or(0),
                bans: b.unwrap_or(0),
            })
            .collect())
    }

    async fn get_peak_hours(
        &self,
        guild_id: Option<&str>,
        days: i32,
    ) -> Result<Vec<PeakActivity>, DomainError> {
        let rows: Vec<(i16, Option<f64>, Option<f64>)> = if let Some(gid) = guild_id {
            sqlx::query_as(
                "SELECT hour, AVG(messages)::float8 AS avg_msg, AVG(infractions)::float8 AS avg_infr \
                 FROM hourly_activity WHERE guild_id = $1 AND day >= CURRENT_DATE - $2::integer\
                 GROUP BY hour ORDER BY avg_msg DESC",
            )
            .bind(gid)
            .bind(days)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                "SELECT hour, AVG(messages)::float8 AS avg_msg, AVG(infractions)::float8 AS avg_infr \
                 FROM hourly_activity WHERE day >= CURRENT_DATE - $1::integer\
                 GROUP BY hour ORDER BY avg_msg DESC",
            )
            .bind(days)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(hour, avg_msg, avg_infr)| PeakActivity {
                hour,
                avg_messages: avg_msg.unwrap_or(0.0),
                avg_infractions: avg_infr.unwrap_or(0.0),
            })
            .collect())
    }

    async fn record_hourly(
        &self,
        guild_id: &str,
        hour: i16,
        messages: i64,
        infractions: i32,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO hourly_activity (guild_id, day, hour, messages, infractions) \
             VALUES ($1, CURRENT_DATE, $2, $3, $4) \
             ON CONFLICT (guild_id, day, hour) DO UPDATE SET \
             messages = hourly_activity.messages + EXCLUDED.messages, \
             infractions = hourly_activity.infractions + EXCLUDED.infractions",
        )
        .bind(guild_id)
        .bind(hour)
        .bind(messages)
        .bind(infractions)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn reset_activity(&self, guild_id: &str) -> Result<u64, DomainError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Internal(format!("reset_activity begin: {e}")))?;
        let h = sqlx::query("DELETE FROM hourly_activity WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::Internal(format!("reset hourly_activity: {e}")))?
            .rows_affected();
        let d = sqlx::query("DELETE FROM daily_activity WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::Internal(format!("reset daily_activity: {e}")))?
            .rows_affected();
        // Vide aussi la baseline : sans ça, le prochain snapshot calculerait un
        // delta basé sur l'ancienne baseline et reproduirait des chiffres faux.
        let b = sqlx::query("DELETE FROM analytics_daily_baseline WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&mut *tx).await
            .map_err(|e| DomainError::Internal(format!("reset analytics_daily_baseline: {e}")))?
            .rows_affected();
        tx.commit().await
            .map_err(|e| DomainError::Internal(format!("reset_activity commit: {e}")))?;
        Ok(h + d + b)
    }
}

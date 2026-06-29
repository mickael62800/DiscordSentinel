use crate::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;

use crate::ports::outbound::community::member_repository::MemberRepository;
use sentinel_core::domain::entities::community::guild_member::GuildMember;
use sentinel_core::domain::errors::DomainError;

pub struct PgMemberRepository {
    pool: PgPool,
}

impl PgMemberRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    guild_id: String,
    user_id: String,
    username: String,
    display_name: Option<String>,
    avatar: Option<String>,
    roles: serde_json::Value,
    joined_at: Option<DateTime<Utc>>,
    account_created: Option<DateTime<Utc>>,
    is_bot: Option<bool>,
    last_seen_at: Option<DateTime<Utc>>,
    left_at: Option<DateTime<Utc>>,
}

impl From<MemberRow> for GuildMember {
    fn from(r: MemberRow) -> Self {
        Self {
            guild_id: r.guild_id.into(),
            user_id: r.user_id.into(),
            username: r.username,
            display_name: r.display_name,
            avatar: r.avatar,
            roles: r.roles,
            joined_at: r.joined_at,
            account_created: r.account_created,
            is_bot: r.is_bot.unwrap_or(false),
            last_seen_at: r.last_seen_at,
            left_at: r.left_at,
        }
    }
}

#[async_trait]
impl MemberRepository for PgMemberRepository {
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<GuildMember>, DomainError> {
        let rows = sqlx::query_as::<_, MemberRow>(
            "SELECT guild_id, user_id, username, display_name, avatar, roles, joined_at, account_created, is_bot, last_seen_at, left_at
             FROM guild_members WHERE guild_id = $1 ORDER BY username ASC"
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("find_members"))?;

        Ok(rows.into_iter().map(GuildMember::from).collect())
    }

    async fn find_one(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<GuildMember>, DomainError> {
        let row = sqlx::query_as::<_, MemberRow>(
            "SELECT guild_id, user_id, username, display_name, avatar, roles, joined_at, account_created, is_bot, last_seen_at, left_at
             FROM guild_members WHERE guild_id = $1 AND user_id = $2"
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find_member"))?;

        Ok(row.map(GuildMember::from))
    }

    async fn upsert(&self, member: &GuildMember) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO guild_members (guild_id, user_id, username, display_name, avatar, roles, joined_at, account_created, is_bot, last_seen_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
             ON CONFLICT (guild_id, user_id) DO UPDATE SET
                username = EXCLUDED.username,
                display_name = EXCLUDED.display_name,
                avatar = EXCLUDED.avatar,
                roles = EXCLUDED.roles,
                joined_at = COALESCE(EXCLUDED.joined_at, guild_members.joined_at),
                account_created = COALESCE(EXCLUDED.account_created, guild_members.account_created),
                is_bot = EXCLUDED.is_bot,
                last_seen_at = NOW()"
        )
        .bind(member.guild_id.as_str())
        .bind(member.user_id.as_str())
        .bind(&member.username)
        .bind(&member.display_name)
        .bind(&member.avatar)
        .bind(&member.roles)
        .bind(member.joined_at)
        .bind(member.account_created)
        .bind(member.is_bot)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("upsert_member"))?;
        Ok(())
    }

    async fn upsert_many(&self, members: &[GuildMember]) -> Result<u64, DomainError> {
        if members.is_empty() {
            return Ok(0);
        }

        let total = members.len();
        tracing::info!(count = total, "Debut sync batch membres");

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(pg_ctx("begin tx upsert_many"))?;

        let mut count = 0u64;
        for member in members {
            sqlx::query(
                "INSERT INTO guild_members (guild_id, user_id, username, display_name, avatar, roles, joined_at, account_created, is_bot, last_seen_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
                 ON CONFLICT (guild_id, user_id) DO UPDATE SET
                    username = EXCLUDED.username,
                    display_name = EXCLUDED.display_name,
                    avatar = EXCLUDED.avatar,
                    roles = EXCLUDED.roles,
                    joined_at = COALESCE(EXCLUDED.joined_at, guild_members.joined_at),
                    account_created = COALESCE(EXCLUDED.account_created, guild_members.account_created),
                    is_bot = EXCLUDED.is_bot,
                    last_seen_at = NOW()"
            )
            .bind(member.guild_id.as_str())
            .bind(member.user_id.as_str())
            .bind(&member.username)
            .bind(&member.display_name)
            .bind(&member.avatar)
            .bind(&member.roles)
            .bind(member.joined_at)
            .bind(member.account_created)
            .bind(member.is_bot)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Internal(format!("upsert_many member {}: {e}", member.user_id)))?;
            count += 1;
        }

        tx.commit().await.map_err(pg_ctx("commit tx upsert_many"))?;

        tracing::info!(synced = count, "Sync batch membres terminee");
        Ok(count)
    }

    async fn delete(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM guild_members WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("delete_member"))?;
        Ok(())
    }

    async fn update_last_seen(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE guild_members SET last_seen_at = NOW() WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("update_last_seen"))?;
        Ok(())
    }

    async fn is_left(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError> {
        // Only true if a row exists AND left_at is set. Pas de ligne -> false (actif).
        let row: Option<(bool,)> = sqlx::query_as(
            "SELECT (left_at IS NOT NULL) FROM guild_members \
             WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("is_left"))?;
        Ok(row.map(|(b,)| b).unwrap_or(false))
    }
}

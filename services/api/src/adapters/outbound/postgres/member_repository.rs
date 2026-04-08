use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::entities::GuildMember;
use crate::domain::errors::DomainError;
use crate::ports::outbound::MemberRepository;

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
}

impl From<MemberRow> for GuildMember {
    fn from(r: MemberRow) -> Self {
        Self {
            guild_id: r.guild_id,
            user_id: r.user_id,
            username: r.username,
            display_name: r.display_name,
            avatar: r.avatar,
            roles: r.roles,
            joined_at: r.joined_at,
            account_created: r.account_created,
            is_bot: r.is_bot.unwrap_or(false),
            last_seen_at: r.last_seen_at,
        }
    }
}

#[async_trait]
impl MemberRepository for PgMemberRepository {
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<GuildMember>, DomainError> {
        let rows = sqlx::query_as::<_, MemberRow>(
            "SELECT guild_id, user_id, username, display_name, avatar, roles, joined_at, account_created, is_bot, last_seen_at
             FROM guild_members WHERE guild_id = $1 ORDER BY username ASC"
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("find_members: {e}")))?;

        Ok(rows.into_iter().map(GuildMember::from).collect())
    }

    async fn find_one(&self, guild_id: &str, user_id: &str) -> Result<Option<GuildMember>, DomainError> {
        let row = sqlx::query_as::<_, MemberRow>(
            "SELECT guild_id, user_id, username, display_name, avatar, roles, joined_at, account_created, is_bot, last_seen_at
             FROM guild_members WHERE guild_id = $1 AND user_id = $2"
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("find_member: {e}")))?;

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
        .bind(&member.guild_id)
        .bind(&member.user_id)
        .bind(&member.username)
        .bind(&member.display_name)
        .bind(&member.avatar)
        .bind(&member.roles)
        .bind(member.joined_at)
        .bind(member.account_created)
        .bind(member.is_bot)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(format!("upsert_member: {e}")))?;
        Ok(())
    }

    async fn upsert_many(&self, members: &[GuildMember]) -> Result<u64, DomainError> {
        if members.is_empty() {
            return Ok(0);
        }

        let total = members.len();
        tracing::info!(count = total, "Debut sync batch membres");

        let mut tx = self.pool.begin().await
            .map_err(|e| DomainError::Internal(format!("begin tx upsert_many: {e}")))?;

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
            .bind(&member.guild_id)
            .bind(&member.user_id)
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

        tx.commit().await
            .map_err(|e| DomainError::Internal(format!("commit tx upsert_many: {e}")))?;

        tracing::info!(synced = count, "Sync batch membres terminee");
        Ok(count)
    }

    async fn delete(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM guild_members WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(format!("delete_member: {e}")))?;
        Ok(())
    }

    async fn update_last_seen(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("UPDATE guild_members SET last_seen_at = NOW() WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(format!("update_last_seen: {e}")))?;
        Ok(())
    }
}

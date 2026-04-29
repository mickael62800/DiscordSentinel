use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::entities::system::guild::Guild;
use crate::domain::errors::DomainError;
use crate::ports::outbound::system::guild_repository::GuildRepository;
use crate::domain::entities::system::discord_ids::GuildId;

pub struct PgGuildRepository {
    pool: PgPool,
}

impl PgGuildRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct GuildRow {
    guild_id: GuildId,
    name: String,
    icon: Option<String>,
    member_count: i32,
    registered_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<GuildRow> for Guild {
    fn from(row: GuildRow) -> Self {
        Self {
            guild_id: row.guild_id,
            name: row.name,
            icon: row.icon,
            member_count: row.member_count,
            registered_at: row.registered_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl GuildRepository for PgGuildRepository {
    async fn upsert(&self, guild: &Guild) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO guilds (guild_id, name, icon, member_count, registered_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (guild_id) DO UPDATE SET
                name = EXCLUDED.name,
                icon = EXCLUDED.icon,
                member_count = EXCLUDED.member_count,
                updated_at = NOW()
            "#,
        )
        .bind(&guild.guild_id)
        .bind(&guild.name)
        .bind(&guild.icon)
        .bind(guild.member_count)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<Guild>, DomainError> {
        let rows = sqlx::query_as::<_, GuildRow>(
            "SELECT * FROM guilds ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(Guild::from).collect())
    }

    async fn find_by_id(&self, guild_id: &str) -> Result<Option<Guild>, DomainError> {
        let row = sqlx::query_as::<_, GuildRow>(
            "SELECT * FROM guilds WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(Guild::from))
    }
}

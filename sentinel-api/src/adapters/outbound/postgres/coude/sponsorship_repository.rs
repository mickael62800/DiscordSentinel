use async_trait::async_trait;
use sqlx::PgPool;

use crate::ports::outbound::coude::sponsorship_repository::Sponsorship;
use crate::ports::outbound::coude::sponsorship_repository::SponsorshipRepository;
use super::super::pg_err;
use sentinel_core::domain::entities::system::discord_ids::GuildId;

pub struct PgSponsorshipRepository { pool: PgPool }

impl PgSponsorshipRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl SponsorshipRepository for PgSponsorshipRepository {
    async fn create(&self, guild_id: &str, sponsor_id: &str, sponsored_id: &str) -> Result<(), sentinel_core::domain::errors::DomainError> {
        sqlx::query(
            "INSERT INTO sponsorships (guild_id, sponsor_id, sponsored_id) \
             VALUES ($1, $2, $3) ON CONFLICT (guild_id, sponsored_id) DO NOTHING",
        )
        .bind(guild_id).bind(sponsor_id).bind(sponsored_id)
        .execute(&self.pool).await.map_err(pg_err)?;
        Ok(())
    }

    async fn list(&self, guild_id: &str) -> Result<Vec<Sponsorship>, sentinel_core::domain::errors::DomainError> {
        #[derive(sqlx::FromRow)]
        struct Row { id: uuid::Uuid, guild_id: String, sponsor_id: String, sponsored_id: String, created_at: chrono::DateTime<chrono::Utc> }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, guild_id, sponsor_id, sponsored_id, created_at \
             FROM sponsorships WHERE guild_id = $1 ORDER BY created_at DESC",
        ).bind(guild_id).fetch_all(&self.pool).await.map_err(pg_err)?;

        Ok(rows.into_iter().map(|r| Sponsorship {
            id: r.id, guild_id: r.guild_id.into(), sponsor_id: r.sponsor_id.into(),
            sponsored_id: r.sponsored_id.into(), created_at: r.created_at,
        }).collect())
    }
}

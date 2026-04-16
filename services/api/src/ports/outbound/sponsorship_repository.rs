use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::errors::DomainError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Sponsorship {
    pub id: Uuid,
    pub guild_id: String,
    pub sponsor_id: String,
    pub sponsored_id: String,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait SponsorshipRepository: Send + Sync {
    async fn create(&self, guild_id: &str, sponsor_id: &str, sponsored_id: &str) -> Result<(), DomainError>;
    async fn list(&self, guild_id: &str) -> Result<Vec<Sponsorship>, DomainError>;
}

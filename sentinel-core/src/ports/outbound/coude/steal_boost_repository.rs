//! Port outbound pour les abonnements boost voleur (Phase 9 Part C).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use crate::domain::entities::coude::steal::boost::StealBoost;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait StealBoostRepository: Send + Sync {
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<StealBoost>, DomainError>;

    async fn upsert(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        days_to_add: i64,
    ) -> Result<DateTime<Utc>, DomainError>;

    async fn purge_expired(&self) -> Result<u64, DomainError>;
}

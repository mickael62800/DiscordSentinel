//! Port outbound pour les abonnements boost voleur (Phase 9 Part C).

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::CoudeStealBoost;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CoudeStealBoostRepository: Send + Sync {
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<CoudeStealBoost>, DomainError>;

    async fn upsert(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        days_to_add: i64,
    ) -> Result<DateTime<Utc>, DomainError>;

    async fn purge_expired(&self) -> Result<u64, DomainError>;
}

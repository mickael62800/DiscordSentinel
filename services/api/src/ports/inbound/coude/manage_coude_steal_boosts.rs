//! Use case des abonnements boost voleur (Phase 9 Part C).

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{CoudeStealBoost, StealBoostDuration};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ManageCoudeStealBoostsUseCase: Send + Sync {
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<CoudeStealBoost>, DomainError>;

    async fn price_for(
        &self,
        item_key: &str,
        duration: StealBoostDuration,
    ) -> Result<i64, DomainError>;

    async fn subscribe(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealBoostDuration,
    ) -> Result<DateTime<Utc>, DomainError>;

    /// Retourne la somme des roll bonuses des items de boost actifs pour
    /// un voleur. Appele par le flow de vol cote bot (via gRPC) pour
    /// ajouter au thief_total avant la comparaison.
    async fn total_bonus(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<i32, DomainError>;
}

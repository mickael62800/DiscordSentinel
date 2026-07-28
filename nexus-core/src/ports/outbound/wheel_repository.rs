//! Port outbound : persistance de la Roue du Destin.

use async_trait::async_trait;

use crate::domain::entities::wheel::WheelSpin;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait WheelRepository: Send + Sync {
    /// Claim atomique du tirage du jour (INSERT ... ON CONFLICT DO NOTHING
    /// dans `nexus_wheel_daily_claims`). Retourne `true` si la row a ete
    /// inseree (premier tirage du jour), `false` si deja claim aujourd'hui.
    async fn try_claim_today(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;

    /// Journalise un spin dans `nexus_wheel_spin_log`.
    async fn log_spin(&self, spin: &WheelSpin) -> Result<(), DomainError>;
}

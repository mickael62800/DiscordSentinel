//! Port outbound pour le log tout-ou-rien (Memorial des clodos).

use async_trait::async_trait;

use crate::domain::entities::coude::tout_ou_rien_log::ToutOuRienLogEntry;
use crate::domain::entities::coude::tout_ou_rien_log::ToutOuRienLogOutcome;
use crate::domain::entities::coude::tout_ou_rien_log::ToutOuRienUserStats;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ToutOuRienRepository: Send + Sync {
    /// Loggue une tentative.
    async fn record(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        mise: i64,
        outcome: ToutOuRienLogOutcome,
        delta: i64,
    ) -> Result<(), DomainError>;

    /// Memorial des clodos : top N pertes (delta le plus negatif).
    async fn memorial(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<ToutOuRienLogEntry>, DomainError>;

    /// Stats agregees d un joueur (pour /profil).
    async fn user_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<ToutOuRienUserStats, DomainError>;
}

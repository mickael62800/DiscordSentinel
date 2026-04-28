//! Port outbound pour les abonnements anti-vol (Phase 9 Part B).

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use crate::domain::entities::coude::steal_protection::CoudeStealProtection;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CoudeStealProtectionRepository: Send + Sync {
    /// Liste les protections actives d'un joueur (expires_at > NOW).
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<CoudeStealProtection>, DomainError>;

    /// Souscrit ou etend un abonnement. Si une protection active existe
    /// deja pour ce couple (guild, user, item_key), on etend `expires_at`
    /// a partir de la valeur actuelle (cumul). Sinon on insere a partir
    /// de NOW.
    ///
    /// Retourne la date d'expiration finale.
    async fn upsert(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        days_to_add: i64,
    ) -> Result<DateTime<Utc>, DomainError>;

    /// Supprime les protections expirees. Utilise par un job de menage
    /// (non critique : la lecture filtre deja sur expires_at > NOW).
    async fn purge_expired(&self) -> Result<u64, DomainError>;
}

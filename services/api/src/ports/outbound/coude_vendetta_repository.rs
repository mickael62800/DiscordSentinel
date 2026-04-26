//! Port outbound pour les vendettas (cf. COUPE_AMELIORATIONS 5.3).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::ActiveVendetta;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CoudeVendettaRepository: Send + Sync {
    /// Declare une nouvelle vendetta. Retourne l UUID.
    async fn declare(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError>;

    /// Vendetta active pour un couple (ordonne) si elle existe.
    async fn get_active(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveVendetta>, DomainError>;

    /// Marque une vendetta comme resolue (won / lost) avec timestamp.
    async fn resolve(
        &self,
        id: Uuid,
        won: bool,
    ) -> Result<(), DomainError>;

    /// Liste les vendettas declarees par un challenger (pour /profil).
    async fn list_by_challenger(
        &self,
        guild_id: &str,
        challenger_id: &str,
    ) -> Result<Vec<ActiveVendetta>, DomainError>;
}

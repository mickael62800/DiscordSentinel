//! Use case vendetta (cf. COUPE_AMELIORATIONS 5.3).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::vendetta::ActiveVendetta;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ManageCoudeVendettaUseCase: Send + Sync {
    /// Declare une vendetta du challenger contre target. Validations :
    /// challenger != target, pas de vendetta deja active sur ce couple.
    /// Retourne l UUID cree.
    async fn declare(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
    ) -> Result<Uuid, DomainError>;

    /// Vendetta active si elle existe (challenger -> target).
    async fn get_active(
        &self,
        guild_id: &str,
        challenger_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveVendetta>, DomainError>;

    /// Marque une vendetta comme resolue.
    async fn resolve(&self, id: Uuid, won: bool) -> Result<(), DomainError>;

    /// Liste les vendettas declarees par un challenger.
    async fn list_by_challenger(
        &self,
        guild_id: &str,
        challenger_id: &str,
    ) -> Result<Vec<ActiveVendetta>, DomainError>;
}

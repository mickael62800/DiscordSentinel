//! Use case du filet de securite (cf. COUPE_AMELIORATIONS 4.4).

use async_trait::async_trait;

use sentinel_core::domain::entities::coude::safety_net::ActiveSafetyNet;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait ManageCoudeSafetyNetUseCase: Send + Sync {
    /// Active le filet pour le joueur si son solde est sous le seuil ET
    /// qu il n a pas deja un filet actif. Retourne `Some(net)` si une
    /// activation a eu lieu, `None` sinon.
    async fn try_activate(
        &self,
        guild_id: &str,
        user_id: &str,
        current_balance: i64,
    ) -> Result<Option<ActiveSafetyNet>, DomainError>;

    /// Retourne le filet actif (si existant et non expire).
    async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<ActiveSafetyNet>, DomainError>;

    /// Liste tous les filets actifs d une guild (pour message quotidien).
    async fn list_active(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ActiveSafetyNet>, DomainError>;
}

//! Port outbound pour le filet de securite (cf. COUPE_AMELIORATIONS 4.4).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::safety_net::ActiveSafetyNet;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CoudeSafetyNetRepository: Send + Sync {
    /// Insere un nouveau filet pour duration_hours. Retourne l UUID.
    async fn activate(
        &self,
        guild_id: &str,
        user_id: &str,
        duration_hours: i64,
    ) -> Result<Uuid, DomainError>;

    /// Retourne le filet actif (expires_at > NOW) si present.
    async fn get_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<ActiveSafetyNet>, DomainError>;

    /// Liste les filets actifs (pour message quotidien). Retourne (user_id,
    /// expires_at).
    async fn list_active(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ActiveSafetyNet>, DomainError>;
}

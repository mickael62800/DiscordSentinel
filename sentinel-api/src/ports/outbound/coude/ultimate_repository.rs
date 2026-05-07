//! Port outbound pour les ultimates par classe (cf. COUPE_AMELIORATIONS 3.1).

use async_trait::async_trait;

use sentinel_core::domain::entities::coude::ultimate::UltimateKind;
use sentinel_core::domain::entities::coude::ultimate::UltimateState;
use sentinel_core::domain::errors::DomainError;

#[async_trait]
pub trait UltimateRepository: Send + Sync {
    /// Active une ultimate (UPSERT). Met `pending_kind` + `last_used_at`
    /// + `activated_at` a NOW.
    async fn activate(
        &self,
        guild_id: &str,
        user_id: &str,
        kind: UltimateKind,
    ) -> Result<(), DomainError>;

    /// Lit l etat d un joueur. Retourne un state vide (None partout) si
    /// jamais utilise.
    async fn get(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UltimateState, DomainError>;

    /// Consume l ultimate pendante (clear pending_kind). Retourne le kind
    /// qui etait pendant (None si rien). Utilise apres l application de
    /// l effet dans le moteur de combat.
    async fn consume_pending(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UltimateKind>, DomainError>;
}

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{BetResolutionPlan, CoudeBet, NewCoudeBet, RefundSummary};
use crate::domain::errors::DomainError;

/// Use case "gérer les paris Coup de Coude".
#[async_trait]
pub trait ManageCoudeBetsUseCase: Send + Sync {
    /// Place un pari. Vérifie que le combat est en phase `betting`, que le parieur
    /// n'est pas un des combattants, que le montant est positif, puis délègue
    /// le reste (lock wallet + débit + insert) au repository.
    async fn place(&self, new: NewCoudeBet) -> Result<(), DomainError>;

    async fn list_for_combat(&self, combat_id: Uuid) -> Result<Vec<CoudeBet>, DomainError>;

    /// Résout les paris d'un combat résolu. `winner_id = None` = égalité (remboursement).
    /// Calcule le plan pari-mutuel côté domaine puis l'applique via le repo.
    /// Retourne le plan pour que le handler puisse l'exposer au client.
    async fn resolve(
        &self,
        combat_id: Uuid,
        winner_id: Option<String>,
    ) -> Result<BetResolutionPlan, DomainError>;

    /// Rembourse les paris non résolus (utilisé quand un combat est annulé).
    async fn refund(&self, combat_id: Uuid) -> Result<RefundSummary, DomainError>;
}

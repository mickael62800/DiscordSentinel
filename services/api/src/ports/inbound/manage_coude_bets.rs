use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{BetResolutionPlan, CoudeBet, NewCoudeBet, RefundSummary, TauntEvent};
use crate::domain::errors::DomainError;

/// Resultat d'un `place` : liste des TauntEvents declenches (faillite du
/// parieur s'il passe a zero apres le debit).
#[derive(Debug, Clone)]
pub struct PlaceBetOutcome {
    pub taunt_events: Vec<TauntEvent>,
}

/// Resultat d'un `resolve` : plan + TauntEvents declenches apres
/// application (jackpots sur payouts + bonus combattants).
#[derive(Debug, Clone)]
pub struct ResolveBetsOutcome {
    pub plan: BetResolutionPlan,
    pub taunt_events: Vec<TauntEvent>,
}

/// Use case "gérer les paris Coup de Coude".
#[async_trait]
pub trait ManageCoudeBetsUseCase: Send + Sync {
    /// Place un pari. Vérifie que le combat est en phase `betting`, que le parieur
    /// n'est pas un des combattants, que le montant est positif, puis délègue
    /// le reste (lock wallet + débit + insert) au repository.
    ///
    /// Retourne la liste des `TauntEvent` declenches (faillite parieur).
    async fn place(&self, new: NewCoudeBet) -> Result<PlaceBetOutcome, DomainError>;

    async fn list_for_combat(&self, combat_id: Uuid) -> Result<Vec<CoudeBet>, DomainError>;

    /// Résout les paris d'un combat résolu. `winner_id = None` = égalité (remboursement).
    /// Calcule le plan pari-mutuel côté domaine puis l'applique via le repo.
    /// Retourne le plan + les taunts declenches (jackpots parieurs / bonus
    /// combattants).
    async fn resolve(
        &self,
        combat_id: Uuid,
        winner_id: Option<String>,
    ) -> Result<ResolveBetsOutcome, DomainError>;

    /// Rembourse les paris non résolus (utilisé quand un combat est annulé).
    async fn refund(&self, combat_id: Uuid) -> Result<RefundSummary, DomainError>;
}

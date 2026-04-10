use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::{
    BetResolutionPlan, CoudeBet, NewCoudeBet, RefundSummary,
};
use crate::domain::errors::DomainError;

/// Repository pour les paris Coup de Coude (`coude_bets`).
///
/// Certaines méthodes traversent la frontière `coude_bets`/`coude_players`
/// pour garantir l'atomicité (verrou pessimiste + débit + insertion). C'est
/// intentionnel : l'alternative (exposer un unit-of-work) est plus coûteuse.
#[async_trait]
pub trait CoudeBetRepository: Send + Sync {
    /// Liste tous les paris d'un combat donné.
    async fn list_for_combat(&self, combat_id: Uuid) -> Result<Vec<CoudeBet>, DomainError>;

    /// Place un pari de manière atomique :
    /// 1. `SELECT ... FOR UPDATE` sur le joueur parieur pour locker son wallet.
    /// 2. Vérifie le solde et renvoie `ValidationError` si insuffisant.
    /// 3. Débite le joueur.
    /// 4. Insère la ligne dans `coude_bets`.
    ///
    /// Note : les vérifications métier (statut du combat, bettor ≠ combattant)
    /// sont à la charge du service **avant** l'appel.
    async fn place(&self, new: NewCoudeBet) -> Result<(), DomainError>;

    /// Applique un plan de résolution calculé par le domaine :
    /// - Crédite les parieurs gagnants (`coins + payout`, `total_earned + payout`).
    /// - Pour les paris gagnants : crédite uniquement si payout > 0, sans toucher `total_earned`
    ///   quand le montant est 0.
    /// - Marque chaque ligne `coude_bets` avec `won`/`payout`.
    /// - Crédite les deux combattants avec leur bonus si > 0.
    ///
    /// Tout est exécuté dans une seule transaction.
    async fn apply_resolution(
        &self,
        guild_id: &str,
        plan: BetResolutionPlan,
    ) -> Result<(), DomainError>;

    /// Rembourse tous les paris non encore résolus d'un combat (`won IS NULL`)
    /// et renvoie le résumé (nombre de lignes + total remboursé).
    /// Utilisé quand un combat est annulé avant sa résolution.
    async fn refund_unresolved(
        &self,
        guild_id: &str,
        combat_id: Uuid,
    ) -> Result<RefundSummary, DomainError>;
}

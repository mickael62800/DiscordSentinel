use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::bet::BetResolutionPlan;
use crate::domain::entities::coude::bet::CoudeBet;
use crate::domain::entities::coude::bet::NewCoudeBet;
use crate::domain::entities::coude::bet::RefundSummary;
use crate::domain::entities::coude::taunt::TauntEvent;
use crate::domain::errors::DomainError;

/// Repository pour les paris Coup de Coude (`coude_bets`).
///
/// Certaines méthodes traversent la frontière `coude_bets`/`user_wallets`
/// pour garantir l'atomicité (verrou pessimiste + débit + insertion). C'est
/// intentionnel : l'alternative (exposer un unit-of-work) est plus coûteuse.
///
/// # Migration wallet unifie (Migration #7)
///
/// Les mutations `user_wallets` dans `place`, `apply_resolution` et
/// `refund_unresolved` passent desormais par `ManageWalletUseCase::{credit_tx,
/// debit_tx}` (variantes qui partagent la tx composite du repo). Les taunts
/// (faillite parieur au debit, jackpot parieur sur gros payout) sont
/// collectes apres `tx.commit()` via `post_commit_taunts` et retournes au
/// service qui les propage dans ses DTOs.
#[async_trait]
pub trait CoudeBetRepository: Send + Sync {
    /// Liste tous les paris d'un combat donné.
    async fn list_for_combat(&self, combat_id: Uuid) -> Result<Vec<CoudeBet>, DomainError>;

    /// Place un pari de manière atomique :
    /// 1. `SELECT ... FOR UPDATE` sur le combat pour verrouiller son statut.
    /// 2. `SELECT ... FOR UPDATE` sur `user_wallets` via `debit_tx` qui
    ///    verifie le solde et echoue en `ValidationError` si insuffisant.
    /// 3. Débite le joueur via `wallet_uc.debit_tx`.
    /// 4. Insère la ligne dans `coude_bets`.
    ///
    /// Retourne la liste de `TauntEvent` declenches apres commit (faillite
    /// cote parieur, detectee par le service wallet).
    ///
    /// Note : les vérifications métier (statut du combat, bettor ≠ combattant)
    /// sont à la charge du service **avant** l'appel.
    async fn place(&self, new: NewCoudeBet) -> Result<Vec<TauntEvent>, DomainError>;

    /// Applique un plan de résolution calculé par le domaine :
    /// - Crédite les parieurs gagnants (`coins + payout`, `total_earned + payout`) via `wallet_uc.credit_tx`.
    /// - Pour les paris gagnants : crédite uniquement si payout > 0, sans toucher `total_earned`
    ///   quand le montant est 0.
    /// - Pour les refunds egalite (won=false, payout=mise) : credit brut de
    ///   `user_wallets` **sans** `total_earned` (preserve la semantique legacy).
    /// - Marque chaque ligne `coude_bets` avec `won`/`payout`.
    /// - Crédite les deux combattants avec leur bonus si > 0 via `wallet_uc.credit_tx`.
    ///
    /// Tout est exécuté dans une seule transaction. Retourne la liste des
    /// `TauntEvent` declenches apres commit (jackpot cote parieurs gagnants
    /// et combattants bonuses).
    async fn apply_resolution(
        &self,
        guild_id: &str,
        plan: BetResolutionPlan,
    ) -> Result<Vec<TauntEvent>, DomainError>;

    /// Rembourse tous les paris non encore résolus d'un combat (`won IS NULL`)
    /// et renvoie le résumé (nombre de lignes + total remboursé).
    /// Utilisé quand un combat est annulé avant sa résolution.
    ///
    /// Refund brut sans `total_earned` (argent qui revient, pas un gain) :
    /// les taunts jackpot ne sont pas declenches ici. Pas de taunt propage.
    async fn refund_unresolved(
        &self,
        guild_id: &str,
        combat_id: Uuid,
    ) -> Result<RefundSummary, DomainError>;
}

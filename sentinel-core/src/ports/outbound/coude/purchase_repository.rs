use async_trait::async_trait;

use crate::domain::errors::DomainError;

/// Issue d'une transaction d'achat atomique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseTxOutcome {
    /// Achat commit : wallet debite, item ajoute, cashbox alimentee.
    Purchased {
        /// Solde du wallet apres le debit.
        new_balance: i64,
    },
    /// Solde insuffisant : la transaction a ete annulee, rien n'a change.
    InsufficientFunds {
        /// Solde courant (inchange).
        balance: i64,
    },
}

/// Port de persistance pour l'achat boutique atomique.
///
/// L'adapter Postgres execute, dans UNE seule transaction : verrou du wallet
/// (`SELECT ... FOR UPDATE`), verif solde, debit + ledger, ajout de l'item a
/// l'inventaire, et alimentation de la cashbox communautaire. L'atomicite est
/// garantie par la transaction DB (rollback natif) — aucune compensation
/// applicative n'est necessaire cote appelant.
#[async_trait]
pub trait PurchaseRepository: Send + Sync {
    async fn purchase_item_atomic(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        price: i64,
    ) -> Result<PurchaseTxOutcome, DomainError>;
}

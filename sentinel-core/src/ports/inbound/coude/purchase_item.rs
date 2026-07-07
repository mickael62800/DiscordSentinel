use async_trait::async_trait;

use crate::domain::errors::DomainError;

/// Resultat d'un achat boutique atomique cote serveur.
///
/// Modelise les deux issues metier "normales" (achat OK / solde insuffisant)
/// SANS passer par une erreur : le solde insuffisant n'est pas un bug mais un
/// etat que le bot doit rendre. Les vraies erreurs (item inconnu, DB down)
/// remontent via `DomainError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseResult {
    /// Achat reussi : coins debites, item ajoute, cashbox alimentee — le tout
    /// dans une seule transaction DB atomique.
    Success {
        /// Prix effectivement paye (config serveur).
        price: i64,
        /// Solde du wallet APRES le debit.
        new_balance: i64,
    },
    /// Solde insuffisant : aucune mutation n'a eu lieu.
    InsufficientFunds {
        /// Prix requis (config serveur).
        price: i64,
        /// Solde courant du joueur (inchange).
        balance: i64,
    },
}

/// Use case "acheter un item de la boutique Coup de Coude".
///
/// Toute la logique economique (prix serveur, verif solde, debit wallet, ajout
/// inventaire, alimentation cashbox) est executee cote serveur dans UNE
/// transaction atomique. Le bot n'est qu'un adaptateur mince qui appelle ce
/// use case et rend le resultat.
#[async_trait]
pub trait PurchaseItemUseCase: Send + Sync {
    async fn purchase_item(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
    ) -> Result<PurchaseResult, DomainError>;
}

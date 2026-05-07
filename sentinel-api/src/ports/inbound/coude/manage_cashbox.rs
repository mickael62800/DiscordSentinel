//! Use case caisse communautaire Coup de Coude.
//!
//! Phase 9 : collecte tous les coins "perdus" dans une caisse par guild
//! et redistribue aleatoirement chaque semaine aux joueurs actifs.

use async_trait::async_trait;
use uuid::Uuid;

use sentinel_core::domain::entities::coude::cashbox::CashboxRedistribution;
use sentinel_core::domain::entities::coude::cashbox::CashboxRedistributionEntry;
use sentinel_core::domain::entities::coude::cashbox::CashboxSource;
use sentinel_core::domain::entities::coude::cashbox::Cashbox;
use sentinel_core::domain::errors::DomainError;

/// Resultat d'une redistribution (pour le retour RPC / page web).
#[derive(Debug, Clone)]
pub struct RedistributionOutcome {
    pub redistribution_id: Uuid,
    pub total_amount: i64,
    pub winners: Vec<(String, String, i64)>, // (user_id, username, amount_won)
}

#[async_trait]
pub trait ManageCoudeCashboxUseCase: Send + Sync {
    /// Etat courant de la caisse.
    async fn get_cashbox(&self, guild_id: &str) -> Result<Cashbox, DomainError>;

    /// Deposit d'un montant dans la caisse (appele par tous les flux qui
    /// retirent des coins de l'economie).
    async fn deposit(
        &self,
        guild_id: &str,
        amount: i64,
        source: CashboxSource,
    ) -> Result<(), DomainError>;

    /// Redistribue atomiquement le contenu de la caisse aux joueurs actifs
    /// des N derniers jours (par defaut 7). Tirage aleatoire avec gains
    /// disparates (du plus gros au plus petit) pour l'effet loterie.
    ///
    /// Retourne None si la caisse est vide ou s'il n'y a aucun joueur actif.
    async fn redistribute_weekly(
        &self,
        guild_id: &str,
    ) -> Result<Option<RedistributionOutcome>, DomainError>;

    /// Redistribue toutes les guilds dont la derniere redistribution date de
    /// plus de `min_days_since_last` jours (ou n'a jamais eu lieu). Appele
    /// par le worker hebdo : il tick regulierement mais l'API filtre
    /// interne — le worker n'a pas a connaitre la liste des guilds.
    async fn redistribute_due_guilds(
        &self,
        min_days_since_last: i64,
    ) -> Result<Vec<(String, RedistributionOutcome)>, DomainError>;

    /// Historique des redistributions (pour la page web).
    async fn list_redistributions(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<CashboxRedistribution>, DomainError>;

    async fn list_entries(
        &self,
        redistribution_id: Uuid,
    ) -> Result<Vec<CashboxRedistributionEntry>, DomainError>;
}

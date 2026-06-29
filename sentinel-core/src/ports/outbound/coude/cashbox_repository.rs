//! Port outbound pour la caisse communautaire Coude (Phase 9).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::coude::cashbox::Cashbox;
use crate::domain::entities::coude::cashbox::CashboxRedistribution;
use crate::domain::entities::coude::cashbox::CashboxRedistributionEntry;
use crate::domain::entities::coude::cashbox::CashboxSource;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait CashboxRepository: Send + Sync {
    /// Recupere (ou cree sur demande) l'etat de la caisse d'une guild.
    async fn get_or_create(&self, guild_id: &str) -> Result<Cashbox, DomainError>;

    /// Ajoute des coins a la caisse. Atomic : cree la row si elle n'existe
    /// pas, sinon increment. Met a jour `total_collected` historique.
    async fn deposit(
        &self,
        guild_id: &str,
        amount: i64,
        source: CashboxSource,
    ) -> Result<(), DomainError>;

    /// Vide la caisse atomiquement et retourne le montant. Utilise par le
    /// job worker hebdomadaire avant redistribution.
    async fn claim_all_for_redistribution(&self, guild_id: &str) -> Result<i64, DomainError>;

    /// Retire un montant de la caisse (Phase 10 /braquage). Clamp au
    /// solde courant — jamais de balance negative. Retourne le montant
    /// effectivement retire (peut etre < `amount` si la caisse etait
    /// moins grosse). `total_collected` n'est PAS incremente (c'est une
    /// sortie, pas une entree).
    async fn withdraw(&self, guild_id: &str, amount: i64) -> Result<i64, DomainError>;

    /// Persiste une redistribution terminee + entries gagnantes dans l'audit.
    async fn record_redistribution(
        &self,
        guild_id: &str,
        total_amount: i64,
        entries: Vec<(String, String, i64)>, // (user_id, username, amount_won)
    ) -> Result<Uuid, DomainError>;

    /// Liste des redistributions passees d'une guild (pour la page web).
    async fn list_redistributions(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<CashboxRedistribution>, DomainError>;

    /// Detail des gains d'une redistribution (pour la page web).
    async fn list_entries(
        &self,
        redistribution_id: Uuid,
    ) -> Result<Vec<CashboxRedistributionEntry>, DomainError>;

    /// Liste des joueurs "actifs" d'une guild dans les N derniers jours.
    /// Un joueur est actif s'il a au moins 1 combat (win/loss/draw) ou 1
    /// vol dans la fenetre. Retourne (user_id, username).
    async fn list_active_players(
        &self,
        guild_id: &str,
        days: i64,
    ) -> Result<Vec<(String, String)>, DomainError>;

    /// Liste les guilds dont la caisse est non vide et dont la derniere
    /// redistribution remonte a plus de `min_days_since_last` jours (ou
    /// n'a jamais eu lieu). Utilise par le worker pour decider quelles
    /// guilds redistribuer sans que l'appelant connaisse la liste.
    async fn list_guilds_due_for_redistribution(
        &self,
        min_days_since_last: i64,
    ) -> Result<Vec<String>, DomainError>;
}

//! Use case des abonnements anti-vol (Phase 9 Part B).
//!
//! Remplace l'approche inventory (items a quantite) par un modele
//! temps-base : chaque item est un abonnement qui tient N jours puis
//! expire. Plusieurs items peuvent etre actifs en parallele et rollent
//! dans l'ordre decroissant de block_chance sur une tentative de vol.

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use crate::domain::entities::coude::steal_protection::StealProtection;
use crate::domain::entities::coude::steal_protection::StealProtectionDuration;
use crate::domain::errors::DomainError;

/// Resultat d'une tentative de blocage d'un vol : indique quel item
/// (s'il y en a) a bloque. Utilise par le bot pour afficher un message
/// different selon l'item.
#[derive(Debug, Clone)]
pub struct StealProtectionTrigger {
    pub item_key: String,
    pub item_name: String,
    pub rolled_value: u32,
    pub block_chance_percent: u32,
}

#[async_trait]
pub trait ManageCoudeStealProtectionsUseCase: Send + Sync {
    /// Liste les abonnements actifs d'un joueur.
    async fn list_active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<StealProtection>, DomainError>;

    /// Calcule le prix total d'un abonnement pour l'item + la duree
    /// donnes. Retourne une erreur si l'item n'existe pas dans le
    /// catalog.
    async fn price_for(
        &self,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<i64, DomainError>;

    /// Souscrit ou etend un abonnement pour un joueur.
    ///
    /// Note : NE DEBITE PAS le wallet (c'est fait par le bot avant
    /// d'appeler). Retourne la nouvelle date d'expiration pour que le
    /// bot puisse l'afficher dans la reponse ephemerale.
    async fn subscribe(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        duration: StealProtectionDuration,
    ) -> Result<DateTime<Utc>, DomainError>;

    /// Fait rouler toutes les protections actives de la cible contre une
    /// tentative de vol. Retourne `Some` si une protection a bloque le
    /// vol (la premiere qui reussit), `None` sinon. Les items rollent
    /// dans l'ordre decroissant de block_chance pour donner la priorite
    /// aux meilleurs.
    ///
    /// Contrairement au modele precedent, AUCUN item n'est consomme :
    /// l'abonnement est temps-base et dure jusqu'a expiration.
    async fn try_trigger(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<StealProtectionTrigger>, DomainError>;
}

//! Port outbound : persistance de la Roue du Destin.

use async_trait::async_trait;

use crate::domain::entities::wheel::{WheelCaseData, WheelSpin};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait WheelRepository: Send + Sync {
    /// Reserve un tirage si le delai est ecoule. Atomique.
    ///
    /// Retourne `true` si le tirage est accorde, `false` si le joueur a deja
    /// tire dans les `cooldown_hours` dernieres heures.
    ///
    /// L'atomicite est le point : deux clics simultanes ne doivent accorder
    /// qu'un seul tirage. Un controle en lecture puis une insertion separee
    /// laisseraient passer les deux.
    async fn try_claim(
        &self,
        guild_id: &str,
        user_id: &str,
        cooldown_hours: i64,
    ) -> Result<bool, DomainError>;

    /// Le joueur a-t-il tire dans les `cooldown_hours` dernieres heures ?
    ///
    /// Lecture SEULE, sans effet de bord : sert a afficher l'etat du bouton
    /// avant tout clic. Elle ne remplace pas `try_claim` — deux clics
    /// simultanes passeraient tous deux ce controle, seul le claim atomique
    /// tranche. C'est un confort d'affichage, pas une regle.
    async fn has_claimed_recently(
        &self,
        guild_id: &str,
        user_id: &str,
        cooldown_hours: i64,
    ) -> Result<bool, DomainError>;

    /// Journalise un spin dans `nexus_wheel_spin_log`.
    async fn log_spin(&self, spin: &WheelSpin) -> Result<(), DomainError>;

    /// Les cases definies par ce serveur, dans l'ordre d'affichage.
    ///
    /// Une liste VIDE signifie « ce serveur n'a rien personnalise » et non
    /// « ce serveur n'a pas de roue » : l'appelant retombe alors sur les cases
    /// historiques. C'est ce qui evite de semer dix lignes par guilde.
    async fn list_cases(&self, guild_id: &str) -> Result<Vec<WheelCaseData>, DomainError>;

    /// Remplace INTEGRALEMENT les cases d'un serveur.
    ///
    /// Remplacement et non fusion : l'editeur envoie la roue complete, et une
    /// fusion laisserait vivre une case supprimee a l'ecran. Une liste vide
    /// efface tout et fait revenir la roue historique.
    async fn replace_cases(
        &self,
        guild_id: &str,
        cases: &[WheelCaseData],
    ) -> Result<(), DomainError>;
}

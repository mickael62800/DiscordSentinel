//! Port outbound : persistance de la Roue du Destin.

use async_trait::async_trait;

use crate::domain::entities::wheel::WheelSpin;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait WheelRepository: Send + Sync {
    /// Claim atomique du tirage du jour (INSERT ... ON CONFLICT DO NOTHING
    /// dans `nexus_wheel_daily_claims`). Retourne `true` si la row a ete
    /// inseree (premier tirage du jour), `false` si deja claim aujourd'hui.
    async fn try_claim_today(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;

    /// Le joueur a-t-il deja tire aujourd'hui ?
    ///
    /// Lecture SEULE, sans effet de bord : sert a afficher l'etat du bouton
    /// avant tout clic. Elle ne remplace pas `try_claim_today` — deux clics
    /// simultanes passeraient tous deux ce controle, seul le claim atomique
    /// tranche. C'est un confort d'affichage, pas une regle.
    async fn has_claimed_today(&self, guild_id: &str, user_id: &str)
        -> Result<bool, DomainError>;

    /// Journalise un spin dans `nexus_wheel_spin_log`.
    async fn log_spin(&self, spin: &WheelSpin) -> Result<(), DomainError>;
}

//! Port inbound (use case) pour le tournoi hebdomadaire "Coup de Coude".
//!
//! Le handler HTTP ne fait que parser/mapper : l'assemblage du classement, le
//! calcul des rangs et l'estimation du prize pool vivent dans le service.

use async_trait::async_trait;

use crate::domain::entities::coude::tournament::CurrentTournament;
use crate::domain::entities::coude::tournament::PastTournament;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ManageTournamentsUseCase: Send + Sync {
    /// Etat du tournoi courant : bornes de semaine, classement (top N des gains
    /// nets) et prize pool estime a partir de la caisse communautaire.
    async fn current_tournament(
        &self,
        guild_id: &str,
    ) -> Result<CurrentTournament, DomainError>;

    /// Historique des tournois resolus / en attente d'une guild.
    async fn tournament_history(
        &self,
        guild_id: &str,
    ) -> Result<Vec<PastTournament>, DomainError>;
}

//! Use case : cycle de loi (proposition, vote, cloture par le worker).

use async_trait::async_trait;

use crate::domain::entities::influence::law::Law;
use crate::domain::entities::influence::vote::{Tally, VoteChoice};
use crate::domain::errors::DomainError;

/// Une loi accompagnee de son decompte courant.
#[derive(Debug, Clone)]
pub struct LawState {
    pub law: Law,
    pub tally: Tally,
}

#[async_trait]
pub trait ManageLawsUseCase: Send + Sync {
    /// Propose une loi : ouvre un vote de tous les citoyens jusqu'a l'echeance.
    async fn propose(
        &self,
        guild_id: &str,
        author_user_id: &str,
        author_username: &str,
        title: &str,
        body: &str,
    ) -> Result<LawState, DomainError>;

    /// Enregistre le vote d'un citoyen sur une loi.
    async fn vote(
        &self,
        guild_id: &str,
        law_id: &str,
        user_id: &str,
        username: &str,
        choice: VoteChoice,
    ) -> Result<LawState, DomainError>;

    /// Etat courant d'une loi (rafraichissement).
    async fn get_state(&self, law_id: &str) -> Result<LawState, DomainError>;

    /// Memorise le message Discord d'une loi (pour edition a la cloture).
    async fn set_message(
        &self,
        law_id: &str,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError>;

    /// Cloture toutes les lois arrivees a echeance. Renvoie leurs etats finaux
    /// (pour que l'appelant notifie / edite les messages). Utilise par le worker.
    async fn close_due(&self) -> Result<Vec<LawState>, DomainError>;
}

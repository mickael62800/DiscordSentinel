//! Use case : votes binaires sur motions (creation, vote, cloture).

use async_trait::async_trait;

use crate::domain::entities::influence::motion::Motion;
use crate::domain::entities::influence::vote::{Tally, VoteChoice};
use crate::domain::errors::DomainError;

/// Une motion accompagnee de son decompte courant.
#[derive(Debug, Clone)]
pub struct MotionState {
    pub motion: Motion,
    pub org_name: String,
    pub tally: Tally,
}

#[async_trait]
pub trait ManageVotesUseCase: Send + Sync {
    /// Cree une motion au sein d'une organisation. L'auteur doit en etre membre.
    async fn create_motion(
        &self,
        guild_id: &str,
        org_name: &str,
        creator_user_id: &str,
        creator_username: &str,
        title: &str,
    ) -> Result<MotionState, DomainError>;

    /// Enregistre le vote d'un membre. Renvoie le decompte a jour.
    async fn cast_vote(
        &self,
        guild_id: &str,
        motion_id: &str,
        user_id: &str,
        username: &str,
        choice: VoteChoice,
    ) -> Result<MotionState, DomainError>;

    /// Cloture une motion (auteur uniquement) et calcule le resultat.
    async fn close_motion(
        &self,
        guild_id: &str,
        motion_id: &str,
        user_id: &str,
    ) -> Result<MotionState, DomainError>;

    /// Etat courant d'une motion (pour rafraichir l'affichage).
    async fn get_state(
        &self,
        guild_id: &str,
        motion_id: &str,
    ) -> Result<MotionState, DomainError>;
}

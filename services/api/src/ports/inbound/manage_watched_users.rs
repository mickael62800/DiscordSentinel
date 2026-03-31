use async_trait::async_trait;

use crate::domain::entities::{
    ConductPointsLog, Infraction, ModerationAction, SecurityEvent, UserNote, WatchedUser,
};
use crate::domain::errors::DomainError;

#[derive(Debug)]
pub struct UserDossier {
    pub user: WatchedUser,
    pub infractions: Vec<Infraction>,
    pub moderation_actions: Vec<ModerationAction>,
    pub security_events: Vec<SecurityEvent>,
    pub conduct_log: Vec<ConductPointsLog>,
    pub notes: Vec<UserNote>,
}

#[async_trait]
pub trait ManageWatchedUsersUseCase: Send + Sync {
    async fn list_watched_users(
        &self,
        guild_id: Option<&str>,
    ) -> Result<Vec<WatchedUser>, DomainError>;

    async fn get_user_dossier(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserDossier, DomainError>;
}

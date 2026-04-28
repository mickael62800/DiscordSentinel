use async_trait::async_trait;

use crate::domain::entities::community::conduct::ConductPointsLog;
use crate::domain::entities::moderation::infraction::Infraction;
use crate::domain::entities::moderation::moderation_action::ModerationAction;
use crate::domain::entities::audit::security_event::SecurityEvent;
use crate::domain::entities::moderation::user_note::UserNote;
use crate::domain::entities::audit::watched_user::WatchedUser;
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
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WatchedUser>, DomainError>;

    async fn get_user_dossier(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserDossier, DomainError>;

    async fn add_manual_watch(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        reason: &str,
    ) -> Result<(), DomainError>;

    async fn remove_manual_watch(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;
}

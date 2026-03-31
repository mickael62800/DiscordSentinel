use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::WatchedUser;
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_watched_users::{ManageWatchedUsersUseCase, UserDossier};
use crate::ports::inbound::{InfractionFilters, ManageInfractionsUseCase, ManageModerationUseCase, ManageNotesUseCase, ManageSecurityUseCase, ManageConductUseCase};
use crate::ports::outbound::WatchedUserRepository;

pub struct ManageWatchedUsersService {
    watched_repo: Arc<dyn WatchedUserRepository>,
    infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    moderation_uc: Arc<dyn ManageModerationUseCase>,
    security_uc: Arc<dyn ManageSecurityUseCase>,
    conduct_uc: Arc<dyn ManageConductUseCase>,
    notes_uc: Arc<dyn ManageNotesUseCase>,
}

impl ManageWatchedUsersService {
    pub fn new(
        watched_repo: Arc<dyn WatchedUserRepository>,
        infractions_uc: Arc<dyn ManageInfractionsUseCase>,
        moderation_uc: Arc<dyn ManageModerationUseCase>,
        security_uc: Arc<dyn ManageSecurityUseCase>,
        conduct_uc: Arc<dyn ManageConductUseCase>,
        notes_uc: Arc<dyn ManageNotesUseCase>,
    ) -> Self {
        Self {
            watched_repo,
            infractions_uc,
            moderation_uc,
            security_uc,
            conduct_uc,
            notes_uc,
        }
    }
}

#[async_trait]
impl ManageWatchedUsersUseCase for ManageWatchedUsersService {
    async fn list_watched_users(
        &self,
        guild_id: Option<&str>,
    ) -> Result<Vec<WatchedUser>, DomainError> {
        self.watched_repo.find_watched_users(guild_id).await
    }

    async fn get_user_dossier(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserDossier, DomainError> {
        let users = self.watched_repo.find_watched_users(Some(guild_id)).await?;
        let user = users
            .into_iter()
            .find(|u| u.user_id == user_id)
            .ok_or_else(|| DomainError::NotFound(format!("Utilisateur {} introuvable", user_id)))?;

        let filters = InfractionFilters {
            user_id: Some(user_id.to_string()),
            action: None,
            limit: 100,
            offset: 0,
        };
        let infractions = self.infractions_uc.list_infractions(guild_id, filters).await?;

        let history = self.moderation_uc.get_history(guild_id, user_id).await?;

        let all_events = self.security_uc.list_events(Some(guild_id)).await?;
        let security_events: Vec<_> = all_events
            .into_iter()
            .filter(|e| e.user_ids.contains(&user_id.to_string()))
            .collect();

        let conduct_log = self
            .conduct_uc
            .get_points_log(guild_id, user_id, 100)
            .await
            .unwrap_or_default();

        let notes = self.notes_uc.get_notes(guild_id, user_id).await.unwrap_or_default();

        Ok(UserDossier {
            user,
            infractions,
            moderation_actions: history.actions,
            security_events,
            conduct_log,
            notes,
        })
    }
}

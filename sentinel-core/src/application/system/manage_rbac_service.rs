//! Use case CRUD du RBAC applicatif. Les garde-fous metier (anti-lockout,
//! dernier owner, troncature du display_name) vivent ici ; le SQL vit dans
//! `RbacRepository` ; le gate HTTP (`require_role`) reste au handler.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::system::rbac::{
    is_owner_self_demotion, truncate_display_name, would_revoke_last_owner, GuildUserEntry,
    UserRoleGrant,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::system::manage_rbac::{
    GrantRoleCommand, ManageRbacUseCase, RevokeRoleCommand, UpdateRoleCommand,
};
use crate::ports::outbound::system::rbac_repository::RbacRepository;

pub struct ManageRbacService {
    repo: Arc<dyn RbacRepository>,
}

impl ManageRbacService {
    pub fn new(repo: Arc<dyn RbacRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageRbacUseCase for ManageRbacService {
    async fn grant_role(&self, cmd: GrantRoleCommand) -> Result<UserRoleGrant, DomainError> {
        let display_name =
            truncate_display_name(cmd.display_name.as_deref().unwrap_or("user"));

        // Upsert api_users pour garantir la FK.
        self.repo.upsert_user(&cmd.user_id, &display_name).await?;

        let granted_at = self
            .repo
            .insert_grant(&cmd.user_id, &cmd.guild_id, cmd.role.as_str(), &cmd.granted_by)
            .await?
            .ok_or_else(|| {
                DomainError::ValidationError(
                    "user a deja un role sur cette guild, utiliser PATCH pour modifier".into(),
                )
            })?;

        Ok(UserRoleGrant {
            discord_user_id: cmd.user_id,
            guild_id: cmd.guild_id,
            role: cmd.role.as_str().to_string(),
            granted_at,
            granted_by: Some(cmd.granted_by),
        })
    }

    async fn update_role(&self, cmd: UpdateRoleCommand) -> Result<(), DomainError> {
        // Garde-fou : anti-lockout du dernier owner-caller.
        if is_owner_self_demotion(&cmd.caller_id, &cmd.user_id, cmd.role.as_str()) {
            return Err(DomainError::ValidationError(
                "un owner ne peut pas se retrograder (lockout risk)".into(),
            ));
        }

        let affected = self
            .repo
            .update_role(&cmd.user_id, &cmd.guild_id, cmd.role.as_str())
            .await?;
        if affected == 0 {
            return Err(DomainError::NotFound(
                "user n'a pas de role sur cette guild".into(),
            ));
        }
        Ok(())
    }

    async fn revoke_role(&self, cmd: RevokeRoleCommand) -> Result<(), DomainError> {
        // Garde-fou : on ne peut pas revoquer le dernier owner d'une guild.
        let total_owners = self.repo.count_owners(&cmd.guild_id).await?;
        let is_target_owner = self.repo.is_owner(&cmd.user_id, &cmd.guild_id).await?;
        if would_revoke_last_owner(is_target_owner, total_owners) {
            return Err(DomainError::ValidationError(
                "impossible de revoquer le dernier owner de la guild".into(),
            ));
        }

        let affected = self.repo.delete_grant(&cmd.user_id, &cmd.guild_id).await?;
        if affected == 0 {
            return Err(DomainError::NotFound(
                "user n'a pas de role sur cette guild".into(),
            ));
        }
        Ok(())
    }

    async fn list_guild_users(
        &self,
        guild_id: &str,
    ) -> Result<Vec<GuildUserEntry>, DomainError> {
        self.repo.list_guild_users(guild_id).await
    }

    async fn is_whitelisted(&self, user_id: &str) -> Result<bool, DomainError> {
        self.repo.is_whitelisted(user_id).await
    }

    async fn ensure_owner_grant(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<(), DomainError> {
        self.repo.grant_owner_if_absent(owner_id, guild_id).await
    }
}

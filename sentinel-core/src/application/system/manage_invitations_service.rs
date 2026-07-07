//! Use case Invitations : generation de code unique, calcul d'expiration,
//! octroi de role atomique au redeem. Toute la regle metier vit ici ; le SQL
//! dans `InvitationRepository`, le handler HTTP ne fait que parse/RBAC/map.

use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;

use crate::domain::entities::system::invitation::{
    AccessStatus, Invitation, RedeemedInvitation, VALID_INVITATION_ROLES,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::system::manage_invitations::{
    CreateInvitationCommand, ManageInvitationsUseCase,
};
use crate::ports::outbound::system::invitation_repository::InvitationRepository;

pub struct ManageInvitationsService {
    repo: Arc<dyn InvitationRepository>,
}

impl ManageInvitationsService {
    pub fn new(repo: Arc<dyn InvitationRepository>) -> Self {
        Self { repo }
    }
}

/// Genere un code aleatoire format XXXX-XXXX-XXXX (12 chars + 2 tirets,
/// 32^12 ≈ 1.2e18 d'entropie).
fn generate_code() -> String {
    // Sans 0/O/1/I/L pour la lisibilite.
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    let mut parts = Vec::with_capacity(3);
    for _ in 0..3 {
        let s: String = (0..4)
            .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
            .collect();
        parts.push(s);
    }
    parts.join("-")
}

#[async_trait]
impl ManageInvitationsUseCase for ManageInvitationsService {
    async fn create_invitation(
        &self,
        cmd: CreateInvitationCommand,
    ) -> Result<Invitation, DomainError> {
        if !VALID_INVITATION_ROLES.contains(&cmd.role.as_str()) {
            return Err(DomainError::ValidationError(format!(
                "role invalide : {}",
                cmd.role
            )));
        }

        // Genere un code, retry si collision (extremement improbable).
        let mut code = generate_code();
        for _ in 0..5 {
            if !self.repo.code_exists(&code).await? {
                break;
            }
            code = generate_code();
        }

        let expires_at: Option<chrono::DateTime<chrono::Utc>> = match cmd.expires_in_hours {
            Some(0) => None, // 0 = pas d'expiration
            Some(h) => Some(chrono::Utc::now() + chrono::Duration::hours(h)),
            None => Some(chrono::Utc::now() + chrono::Duration::hours(168)), // defaut 7j
        };

        self.repo
            .insert_invitation(
                &code,
                &cmd.guild_id,
                &cmd.role,
                &cmd.created_by,
                expires_at,
                cmd.notes.as_deref(),
            )
            .await?;

        Ok(Invitation {
            code,
            guild_id: cmd.guild_id,
            role: cmd.role,
            created_by: cmd.created_by,
            created_at: chrono::Utc::now(),
            expires_at,
            used_at: None,
            used_by_discord_id: None,
            notes: cmd.notes,
        })
    }

    async fn list_invitations(&self, guild_id: &str) -> Result<Vec<Invitation>, DomainError> {
        self.repo.list_by_guild(guild_id).await
    }

    async fn find_invitation(&self, code: &str) -> Result<Option<Invitation>, DomainError> {
        self.repo.find_by_code(code).await
    }

    async fn revoke_invitation(&self, code: &str) -> Result<(), DomainError> {
        self.repo.delete_unused(code).await
    }

    async fn check_access(
        &self,
        discord_user_id: &str,
        is_superadmin: bool,
    ) -> Result<AccessStatus, DomainError> {
        let guild_count = self.repo.count_user_guilds(discord_user_id).await?;
        let is_authorized = is_superadmin || guild_count > 0;
        let message = if is_authorized {
            "Acces autorise".to_string()
        } else {
            "Acces non autorise. Utilise un code d'invitation pour rejoindre.".to_string()
        };
        Ok(AccessStatus {
            is_authorized,
            is_superadmin,
            guild_count,
            message,
        })
    }

    async fn redeem_invitation(
        &self,
        discord_user_id: &str,
        code: &str,
    ) -> Result<RedeemedInvitation, DomainError> {
        let code = code.trim().to_uppercase();
        if code.is_empty() {
            return Err(DomainError::ValidationError("code vide".into()));
        }

        let Some(inv) = self.repo.find_by_code(&code).await? else {
            return Err(DomainError::NotFound("code invalide ou inexistant".into()));
        };

        if inv.used_at.is_some() {
            return Err(DomainError::Conflict(
                "code deja utilise par un autre utilisateur".into(),
            ));
        }
        if let Some(exp) = inv.expires_at {
            if exp < chrono::Utc::now() {
                return Err(DomainError::Conflict("code expire".into()));
            }
        }

        // Octroi du role + consommation du code de facon atomique.
        let consumed = self
            .repo
            .redeem(&code, discord_user_id, &inv.guild_id, &inv.role)
            .await?;

        if !consumed {
            // Course : un autre user a consomme le code entre-temps.
            return Err(DomainError::Conflict(
                "race : code consomme par un autre utilisateur".into(),
            ));
        }

        tracing::info!(
            target: "audit::invitation",
            actor = %discord_user_id,
            guild_id = %inv.guild_id,
            role = %inv.role,
            code = %code,
            "invitation redeemed"
        );

        Ok(RedeemedInvitation {
            guild_id: inv.guild_id,
            role: inv.role,
        })
    }
}

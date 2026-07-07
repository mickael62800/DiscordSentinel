use async_trait::async_trait;

use crate::domain::entities::system::invitation::{
    AccessStatus, Invitation, RedeemedInvitation,
};
use crate::domain::errors::DomainError;

/// Commande de creation d'un code d'invitation.
pub struct CreateInvitationCommand {
    pub guild_id: String,
    pub role: String,
    /// Heures avant expiration (defaut 168 = 7 jours, `Some(0)` = pas d'expiration).
    pub expires_in_hours: Option<i64>,
    pub notes: Option<String>,
    /// Discord user id du createur (RBAC deja verifie par le handler).
    pub created_by: String,
}

#[async_trait]
pub trait ManageInvitationsUseCase: Send + Sync {
    /// Genere un code unique (retry anti-collision), calcule l'expiration et
    /// persiste l'invitation. Valide que le role fait partie des roles autorises.
    async fn create_invitation(
        &self,
        cmd: CreateInvitationCommand,
    ) -> Result<Invitation, DomainError>;

    /// Liste les invitations d'une guild (tri par date de creation desc).
    async fn list_invitations(&self, guild_id: &str) -> Result<Vec<Invitation>, DomainError>;

    /// Recupere une invitation par code (pour la resolution RBAC cote handler).
    async fn find_invitation(&self, code: &str) -> Result<Option<Invitation>, DomainError>;

    /// Revoque (supprime) un code non utilise. No-op si deja consomme.
    async fn revoke_invitation(&self, code: &str) -> Result<(), DomainError>;

    /// Evalue l'autorisation d'acces d'un user (superadmin ou membre d'au moins
    /// une guild).
    async fn check_access(
        &self,
        discord_user_id: &str,
        is_superadmin: bool,
    ) -> Result<AccessStatus, DomainError>;

    /// Consomme un code : octroie le role sur la guild et marque le code utilise
    /// de facon atomique. Renvoie la guild + le role octroyes.
    async fn redeem_invitation(
        &self,
        discord_user_id: &str,
        code: &str,
    ) -> Result<RedeemedInvitation, DomainError>;
}

use async_trait::async_trait;

use crate::domain::entities::system::invitation::Invitation;
use crate::domain::errors::DomainError;

/// Port sortant : persistance des codes d'invitation (table `invitation_codes`)
/// et octroi de role (table `api_user_guilds`).
#[async_trait]
pub trait InvitationRepository: Send + Sync {
    /// `true` si un code identique existe deja (anti-collision).
    async fn code_exists(&self, code: &str) -> Result<bool, DomainError>;

    /// Insere une nouvelle invitation.
    async fn insert_invitation(
        &self,
        code: &str,
        guild_id: &str,
        role: &str,
        created_by: &str,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        notes: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Liste les invitations d'une guild (creees en dernier d'abord).
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Invitation>, DomainError>;

    /// Recupere une invitation par son code.
    async fn find_by_code(&self, code: &str) -> Result<Option<Invitation>, DomainError>;

    /// Supprime un code s'il n'est pas encore utilise (`used_at IS NULL`).
    async fn delete_unused(&self, code: &str) -> Result<(), DomainError>;

    /// Nombre de guilds pour lesquelles l'utilisateur a un role.
    async fn count_user_guilds(&self, discord_user_id: &str) -> Result<i64, DomainError>;

    /// Transaction atomique : octroie `role` sur `guild_id` a l'utilisateur et
    /// marque le code consomme (`used_at`, `used_by_discord_id`) UNIQUEMENT si le
    /// code etait encore libre. Renvoie `true` si le code a bien ete consomme,
    /// `false` en cas de course (consomme entre-temps par un autre user).
    async fn redeem(
        &self,
        code: &str,
        discord_user_id: &str,
        guild_id: &str,
        role: &str,
    ) -> Result<bool, DomainError>;
}

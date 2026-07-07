//! Port inbound : use case du flux OAuth2 Discord web. La validation CSRF, les
//! cookies et l'echange HTTP avec Discord restent au handler (concern HTTP) ;
//! ce use case ne porte que la persistance (sessions + trace de login).

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::system::oauth::{
    LoginTrace, NewOAuthSession, OAuthSession, SessionTokenUpdate,
};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ManageOAuthUseCase: Send + Sync {
    /// Journalise un login OAuth reussi.
    async fn record_login(&self, trace: LoginTrace) -> Result<(), DomainError>;

    /// Cree une session web persistante (refresh token cote serveur).
    async fn create_session(&self, session: NewOAuthSession) -> Result<(), DomainError>;

    /// Relit une session par son id opaque (cookie).
    async fn get_session(&self, id: Uuid) -> Result<Option<OAuthSession>, DomainError>;

    /// Met a jour `last_used_at` (token encore valide).
    async fn touch_session(&self, id: Uuid) -> Result<(), DomainError>;

    /// Remplace les tokens apres un refresh Discord.
    async fn update_tokens(&self, update: SessionTokenUpdate) -> Result<(), DomainError>;

    /// Supprime la session (logout ou refresh refuse).
    async fn delete_session(&self, id: Uuid) -> Result<(), DomainError>;
}

//! Port outbound : persistance des sessions OAuth web + trace des logins.
//! Tout le SQL (`web_oauth_sessions`, `successful_logins`) vit dans l'adapter.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::system::oauth::{
    LoginTrace, NewOAuthSession, OAuthSession, SessionTokenUpdate,
};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait OAuthSessionRepository: Send + Sync {
    /// Journalise un login OAuth reussi (best-effort cote appelant).
    async fn record_login(&self, trace: LoginTrace) -> Result<(), DomainError>;

    /// Cree une session web persistante (refresh token cote serveur).
    async fn create_session(&self, session: NewOAuthSession) -> Result<(), DomainError>;

    /// Relit une session par son id opaque (cookie).
    async fn get_session(&self, id: Uuid) -> Result<Option<OAuthSession>, DomainError>;

    /// Met a jour `last_used_at` (token encore valide, pas de refresh).
    async fn touch_session(&self, id: Uuid) -> Result<(), DomainError>;

    /// Remplace les tokens apres un refresh Discord + `last_used_at`.
    async fn update_tokens(&self, update: SessionTokenUpdate) -> Result<(), DomainError>;

    /// Supprime la session (logout ou refresh refuse par Discord).
    async fn delete_session(&self, id: Uuid) -> Result<(), DomainError>;
}

//! Implementation du use case OAuth web. Pass-through vers le repo : le SQL
//! (sessions + logins) vit dans l'adapter Postgres, l'echange HTTP avec Discord
//! et la logique CSRF/cookies restent au handler.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::system::oauth::{
    LoginTrace, NewOAuthSession, OAuthSession, SessionTokenUpdate,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::system::manage_oauth::ManageOAuthUseCase;
use crate::ports::outbound::system::oauth_session_repository::OAuthSessionRepository;

pub struct ManageOAuthService {
    repo: Arc<dyn OAuthSessionRepository>,
}

impl ManageOAuthService {
    pub fn new(repo: Arc<dyn OAuthSessionRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageOAuthUseCase for ManageOAuthService {
    async fn record_login(&self, trace: LoginTrace) -> Result<(), DomainError> {
        self.repo.record_login(trace).await
    }

    async fn create_session(&self, session: NewOAuthSession) -> Result<(), DomainError> {
        self.repo.create_session(session).await
    }

    async fn get_session(&self, id: Uuid) -> Result<Option<OAuthSession>, DomainError> {
        self.repo.get_session(id).await
    }

    async fn touch_session(&self, id: Uuid) -> Result<(), DomainError> {
        self.repo.touch_session(id).await
    }

    async fn update_tokens(&self, update: SessionTokenUpdate) -> Result<(), DomainError> {
        self.repo.update_tokens(update).await
    }

    async fn delete_session(&self, id: Uuid) -> Result<(), DomainError> {
        self.repo.delete_session(id).await
    }
}

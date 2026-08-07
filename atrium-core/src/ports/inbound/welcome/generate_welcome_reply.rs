use async_trait::async_trait;

use crate::domain::{WelcomeError, WelcomeReply, WelcomeRequest};

#[async_trait]
pub trait GenerateWelcomeReplyUseCase: Send + Sync {
    async fn reply(&self, request: WelcomeRequest) -> Result<WelcomeReply, WelcomeError>;
}

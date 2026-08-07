use async_trait::async_trait;

use crate::domain::WelcomePrompt;

#[derive(Debug, thiserror::Error)]
#[error("le fournisseur IA est indisponible")]
pub struct AiProviderError;

#[async_trait]
pub trait WelcomeAiGateway: Send + Sync {
    async fn generate(&self, prompt: WelcomePrompt) -> Result<String, AiProviderError>;
}

use async_trait::async_trait;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait DiscordApiRepository: Send + Sync {
    /// Upload un emoji custom sur un serveur Discord.
    /// Retourne un tuple `(emoji_id, emoji_name)`.
    async fn upload_emoji(
        &self,
        guild_id: &str,
        name: &str,
        image_bytes: &[u8],
        mime: &str,
    ) -> Result<(String, String), DomainError>;
}

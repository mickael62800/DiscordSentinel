use crate::domain::errors::DomainError;

/// Service pour les appels a l'API Discord.
/// Centralise la logique d'interaction avec Discord (ban, unban, etc.)
pub struct DiscordApiService {
    token: String,
    client: reqwest::Client,
}

impl DiscordApiService {
    pub fn new(token: String) -> Self {
        Self {
            token,
            client: reqwest::Client::new(),
        }
    }

    #[allow(dead_code)]
    pub fn is_configured(&self) -> bool {
        !self.token.is_empty()
    }

    fn ensure_configured(&self) -> Result<(), DomainError> {
        if self.token.is_empty() {
            return Err(DomainError::Internal(
                "MODERATION_DISCORD_TOKEN non configure".into(),
            ));
        }
        Ok(())
    }

    /// Bannir un utilisateur d'un serveur Discord.
    pub async fn ban_user(
        &self,
        guild_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/bans/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .put(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({
                "delete_message_seconds": 86400,
                "reason": reason,
            }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord ban failed ({status}): {body}"
            )));
        }

        Ok(())
    }

    /// Debannir un utilisateur d'un serveur Discord.
    pub async fn unban_user(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        let url = format!(
            "https://discord.com/api/v10/guilds/{}/bans/{}",
            guild_id, user_id
        );

        let resp = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() && status.as_u16() != 404 {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!(
                "Discord unban failed ({status}): {body}"
            )));
        }

        Ok(())
    }
}

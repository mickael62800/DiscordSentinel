use serde::Deserialize;

use crate::domain::errors::DomainError;

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DiscordMember {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

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

    /// Recuperer la liste des membres d'un serveur Discord (id + username).
    pub async fn list_members(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<DiscordMember>, DomainError> {
        self.ensure_configured()?;

        let mut all_members = Vec::new();
        let mut after: Option<String> = None;
        let page_size = std::cmp::min(limit, 1000);

        loop {
            let mut url = format!(
                "https://discord.com/api/v10/guilds/{}/members?limit={}",
                guild_id, page_size
            );
            if let Some(ref after_id) = after {
                url.push_str(&format!("&after={}", after_id));
            }

            let resp = self
                .client
                .get(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .send()
                .await
                .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(DomainError::Internal(format!(
                    "Discord list members failed ({status}): {body}"
                )));
            }

            let members: Vec<serde_json::Value> = resp
                .json()
                .await
                .map_err(|e| DomainError::Internal(format!("Discord parse error: {e}")))?;

            if members.is_empty() {
                break;
            }

            for m in &members {
                let user = match m.get("user") {
                    Some(u) => u,
                    None => continue,
                };

                let id = user.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let username = user.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let display_name = m.get("nick")
                    .and_then(|v| v.as_str())
                    .or_else(|| user.get("global_name").and_then(|v| v.as_str()))
                    .map(|s| s.to_string());

                let avatar_hash = user.get("avatar").and_then(|v| v.as_str());
                let avatar_url = avatar_hash.map(|h| {
                    format!("https://cdn.discordapp.com/avatars/{}/{}.png?size=64", id, h)
                });

                if !id.is_empty() {
                    all_members.push(DiscordMember {
                        id,
                        username,
                        display_name,
                        avatar_url,
                    });
                }
            }

            if all_members.len() >= limit as usize || members.len() < page_size as usize {
                break;
            }

            after = all_members.last().map(|m| m.id.clone());
        }

        Ok(all_members)
    }

    /// Envoyer un message prive a un utilisateur Discord.
    pub async fn send_dm(
        &self,
        user_id: &str,
        content: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;

        // 1. Creer un canal DM
        let dm_resp = self
            .client
            .post("https://discord.com/api/v10/users/@me/channels")
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "recipient_id": user_id }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord DM channel error: {e}")))?;

        if !dm_resp.status().is_success() {
            let body = dm_resp.text().await.unwrap_or_default();
            tracing::warn!("Impossible d'ouvrir un DM avec {user_id}: {body}");
            return Ok(()); // Ne pas faire echouer la suppression si le DM echoue
        }

        let channel: serde_json::Value = dm_resp.json().await
            .map_err(|e| DomainError::Internal(format!("Discord DM parse error: {e}")))?;
        let channel_id = channel["id"].as_str().unwrap_or_default();

        // 2. Envoyer le message
        let msg_resp = self
            .client
            .post(format!("https://discord.com/api/v10/channels/{channel_id}/messages"))
            .header("Authorization", format!("Bot {}", self.token))
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord send DM error: {e}")))?;

        if !msg_resp.status().is_success() {
            let body = msg_resp.text().await.unwrap_or_default();
            tracing::warn!("Echec envoi DM a {user_id}: {body}");
        }

        Ok(())
    }

    // ── Gestion des roles ──

    /// Creer un role Discord.
    pub async fn create_role(
        &self,
        guild_id: &str,
        name: &str,
        color: u32,
        permissions: Option<&str>,
    ) -> Result<serde_json::Value, DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{}/roles", guild_id);

        let mut body = serde_json::json!({
            "name": name,
            "color": color,
            "mentionable": false,
        });
        if let Some(perms) = permissions {
            body["permissions"] = serde_json::Value::String(perms.to_string());
        }

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!("Discord create_role failed: {body}")));
        }

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| DomainError::Internal(format!("Parse error: {e}")))
    }

    /// Modifier un role Discord.
    pub async fn edit_role(
        &self,
        guild_id: &str,
        role_id: &str,
        name: Option<&str>,
        color: Option<u32>,
        permissions: Option<&str>,
        mentionable: Option<bool>,
        hoist: Option<bool>,
    ) -> Result<serde_json::Value, DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{}/roles/{}", guild_id, role_id);

        let mut body = serde_json::json!({});
        if let Some(n) = name { body["name"] = serde_json::Value::String(n.to_string()); }
        if let Some(c) = color { body["color"] = serde_json::json!(c); }
        if let Some(p) = permissions { body["permissions"] = serde_json::Value::String(p.to_string()); }
        if let Some(m) = mentionable { body["mentionable"] = serde_json::json!(m); }
        if let Some(h) = hoist { body["hoist"] = serde_json::json!(h); }

        let resp = self.client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!("Discord edit_role failed: {body}")));
        }

        resp.json::<serde_json::Value>()
            .await
            .map_err(|e| DomainError::Internal(format!("Parse error: {e}")))
    }

    /// Supprimer un role Discord.
    pub async fn delete_role(
        &self,
        guild_id: &str,
        role_id: &str,
    ) -> Result<(), DomainError> {
        self.ensure_configured()?;
        let url = format!("https://discord.com/api/v10/guilds/{}/roles/{}", guild_id, role_id);

        let resp = self.client
            .delete(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .send()
            .await
            .map_err(|e| DomainError::Internal(format!("Discord API error: {e}")))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DomainError::Internal(format!("Discord delete_role failed: {body}")));
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

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

// ── Request DTOs ──

#[derive(Debug, Serialize)]
pub struct CreateVoiceChannelRequest {
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct UpdateVoiceChannelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_limit: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_channel_id: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct TransferOwnershipRequest {
    pub new_owner_id: String,
    pub new_owner_name: String,
}

#[derive(Debug, Serialize)]
pub struct AddCoAdminRequest {
    pub user_id: String,
    pub user_name: String,
}

#[derive(Debug, Serialize)]
pub struct AddWhitelistRequest {
    pub guild_id: String,
    pub owner_id: String,
    pub target_id: String,
    pub target_name: String,
}

#[derive(Debug, Serialize)]
pub struct BanFromChannelRequest {
    pub user_id: String,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub duration_secs: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct LogModerationActionRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub duration: Option<i64>,
}

// ── Response DTOs ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VoiceChannelResponse {
    pub id: String,
    pub guild_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: String,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
    pub locked: bool,
    pub member_limit: Option<i32>,
    pub status: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct WhitelistEntryResponse {
    pub target_id: String,
    pub target_name: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VoiceChannelDetailResponse {
    pub channel: VoiceChannelResponse,
    pub co_admins: Vec<serde_json::Value>,
    pub bans: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BanCheckResponse {
    pub banned: bool,
}

// ── Client ──

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    const BOT_NAME: &'static str = "voice-bot";

    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: config.api_base_url.clone(),
            api_key: config.api_key.clone(),
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(&self.api_key)
        }
    }

    pub async fn heartbeat(&self, name: &str) -> Result<(), String> {
        #[derive(serde::Serialize)]
        struct Payload { name: String }

        let mut req = self.client
            .post(format!("{}/api/bots/heartbeat", self.base_url))
            .json(&Payload { name: name.to_string() });

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send().await.map_err(|e| format!("Heartbeat failed: {e}"))?;
        Ok(())
    }

    pub async fn get_guild_config(&self, guild_id: &str) -> Result<std::collections::HashMap<String, String>, String> {
        let url = format!("{}/api/bots/config/{}/{}", self.base_url, guild_id, Self::BOT_NAME);
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        #[derive(serde::Deserialize)]
        struct ConfigEntry {
            config_key: String,
            config_value: String,
        }

        let resp = req.send().await.map_err(|e| format!("Config fetch failed: {e}"))?;
        let entries: Vec<ConfigEntry> = resp.json().await.map_err(|e| format!("Config parse failed: {e}"))?;
        Ok(entries.into_iter().map(|e| (e.config_key, e.config_value)).collect())
    }

    /// Helper pour lire une valeur de config avec fallback
    pub fn config_or(config: &std::collections::HashMap<String, String>, key: &str, default: &str) -> String {
        config.get(key).cloned().unwrap_or_else(|| default.to_string())
    }

    pub fn config_u64(config: &std::collections::HashMap<String, String>, key: &str, default: u64) -> u64 {
        config.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    // ── Channels ──

    pub async fn list_channels(
        &self,
        guild_id: &str,
    ) -> Result<Vec<VoiceChannelResponse>, String> {
        let req = self
            .client
            .get(format!("{}/api/voice-channels/{guild_id}", self.base_url));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<Vec<VoiceChannelResponse>>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn create_channel(
        &self,
        request: &CreateVoiceChannelRequest,
    ) -> Result<VoiceChannelResponse, String> {
        let req = self
            .client
            .post(format!("{}/api/voice-channels", self.base_url))
            .json(request);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<VoiceChannelResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn delete_channel(&self, channel_id: &str) -> Result<(), String> {
        let req = self.client.delete(format!(
            "{}/api/voice-channels/by-channel/{channel_id}",
            self.base_url
        ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    pub async fn update_channel(
        &self,
        channel_id: &str,
        request: &UpdateVoiceChannelRequest,
    ) -> Result<(), String> {
        let req = self
            .client
            .patch(format!(
                "{}/api/voice-channels/by-channel/{channel_id}",
                self.base_url
            ))
            .json(request);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    pub async fn get_channel(
        &self,
        channel_id: &str,
    ) -> Result<Option<VoiceChannelResponse>, String> {
        let req = self.client.get(format!(
            "{}/api/voice-channels/by-channel/{channel_id}",
            self.base_url
        ));

        let response = self
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        let detail = response
            .json::<VoiceChannelDetailResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))?;

        Ok(Some(detail.channel))
    }

    // ── Transfer ──

    pub async fn transfer_ownership(
        &self,
        channel_id: &str,
        request: &TransferOwnershipRequest,
    ) -> Result<(), String> {
        let req = self
            .client
            .patch(format!(
                "{}/api/voice-channels/by-channel/{channel_id}/transfer",
                self.base_url
            ))
            .json(request);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    // ── Co-admins ──

    pub async fn add_co_admin(
        &self,
        channel_id: &str,
        request: &AddCoAdminRequest,
    ) -> Result<(), String> {
        let req = self
            .client
            .post(format!(
                "{}/api/voice-channels/by-channel/{channel_id}/co-admins",
                self.base_url
            ))
            .json(request);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn remove_co_admin(&self, channel_id: &str, user_id: &str) -> Result<(), String> {
        let req = self.client.delete(format!(
            "{}/api/voice-channels/by-channel/{channel_id}/co-admins/{user_id}",
            self.base_url
        ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    // ── Whitelist ──

    #[allow(dead_code)]
    pub async fn get_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Vec<WhitelistEntryResponse>, String> {
        let req = self.client.get(format!(
            "{}/api/voice-channels/whitelist/{guild_id}/{owner_id}",
            self.base_url
        ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<Vec<WhitelistEntryResponse>>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn add_to_whitelist(&self, request: &AddWhitelistRequest) -> Result<(), String> {
        let req = self
            .client
            .post(format!(
                "{}/api/voice-channels/whitelist",
                self.base_url
            ))
            .json(request);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    // ── Bans ──

    pub async fn ban_user(
        &self,
        channel_id: &str,
        request: &BanFromChannelRequest,
    ) -> Result<(), String> {
        let req = self
            .client
            .post(format!(
                "{}/api/voice-channels/by-channel/{channel_id}/bans",
                self.base_url
            ))
            .json(request);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn unban_user(&self, channel_id: &str, user_id: &str) -> Result<(), String> {
        let req = self.client.delete(format!(
            "{}/api/voice-channels/by-channel/{channel_id}/bans/{user_id}",
            self.base_url
        ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn check_ban(&self, channel_id: &str, user_id: &str) -> Result<bool, String> {
        let req = self.client.get(format!(
            "{}/api/voice-channels/by-channel/{channel_id}/bans/{user_id}",
            self.base_url
        ));

        let resp = self
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<BanCheckResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))?;

        Ok(resp.banned)
    }

    // ── Guild registration ──

    pub async fn register_guild(&self, guild_id: &str, name: &str, member_count: i32) -> Result<(), String> {
        #[derive(serde::Serialize)]
        struct Payload {
            guild_id: String,
            name: String,
            member_count: Option<i32>,
        }

        let mut req = self.client
            .post(format!("{}/api/guilds/register", self.base_url))
            .json(&Payload {
                guild_id: guild_id.to_string(),
                name: name.to_string(),
                member_count: Some(member_count),
            });

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send().await.map_err(|e| format!("Guild register failed: {e}"))?;
        Ok(())
    }

    // ── Moderation (log anti-flood mutes) ──

    pub async fn log_moderation_action(
        &self,
        request: &LogModerationActionRequest,
    ) -> Result<(), String> {
        let req = self
            .client
            .post(format!("{}/api/moderation/actions", self.base_url))
            .json(request);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }
}

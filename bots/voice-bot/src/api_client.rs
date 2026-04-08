use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_shared::api_client::BaseApiClient;

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
pub struct VoiceChannelDetailResponse {
    pub channel: VoiceChannelResponse,
    pub co_admins: Vec<serde_json::Value>,
    pub bans: Vec<serde_json::Value>,
}

// ── Client ──

pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    // ── Channels ──

    pub async fn list_channels(
        &self,
        guild_id: &str,
    ) -> Result<Vec<VoiceChannelResponse>, String> {
        self.base
            .get_json(&format!("/api/voice-channels/{guild_id}"))
            .await
    }

    pub async fn create_channel(
        &self,
        request: &CreateVoiceChannelRequest,
    ) -> Result<VoiceChannelResponse, String> {
        self.base
            .post_json("/api/voice-channels", request)
            .await
    }

    pub async fn delete_channel(&self, channel_id: &str) -> Result<(), String> {
        // DELETE without body — use raw client
        let req = self.base.client().delete(format!(
            "{}/api/voice-channels/by-channel/{channel_id}",
            self.base.base_url()
        ));
        self.base
            .auth(req)
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
        self.base
            .patch_fire_and_forget(
                &format!("/api/voice-channels/by-channel/{channel_id}"),
                request,
            )
            .await;
        Ok(())
    }

    pub async fn get_channel(
        &self,
        channel_id: &str,
    ) -> Result<Option<VoiceChannelResponse>, String> {
        let path = format!("/api/voice-channels/by-channel/{channel_id}");
        let resp = self.base.auth(
            self.base.client().get(format!("{}{}", self.base.base_url(), path))
        ).send().await.map_err(|e| format!("Erreur reseau: {e}"))?;

        if resp.status().as_u16() == 404 {
            return Ok(None);
        }

        let detail = resp
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
        self.base
            .patch_fire_and_forget(
                &format!("/api/voice-channels/by-channel/{channel_id}/transfer"),
                request,
            )
            .await;
        Ok(())
    }

    // ── Co-admins ──

    pub async fn add_co_admin(
        &self,
        channel_id: &str,
        request: &AddCoAdminRequest,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/voice-channels/by-channel/{channel_id}/co-admins"),
                request,
            )
            .await;
        Ok(())
    }

    // ── Whitelist ──

    pub async fn add_to_whitelist(&self, request: &AddWhitelistRequest) -> Result<(), String> {
        self.base
            .post_fire_and_forget("/api/voice-channels/whitelist", request)
            .await;
        Ok(())
    }

    // ── Bans ──

    pub async fn ban_user(
        &self,
        channel_id: &str,
        request: &BanFromChannelRequest,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget(
                &format!("/api/voice-channels/by-channel/{channel_id}/bans"),
                request,
            )
            .await;
        Ok(())
    }

    // ── Moderation (log anti-flood mutes) ──

    pub async fn log_moderation_action(
        &self,
        request: &LogModerationActionRequest,
    ) -> Result<(), String> {
        self.base
            .post_fire_and_forget("/api/moderation/actions", request)
            .await;
        Ok(())
    }
}

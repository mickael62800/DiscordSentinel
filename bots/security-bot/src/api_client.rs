use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;

#[derive(Debug, Serialize)]
pub struct SecurityEvent {
    pub guild_id: String,
    pub event_type: String,
    pub severity: String,
    pub description: String,
    pub user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberPayload {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: serde_json::Value,
    pub joined_at: Option<DateTime<Utc>>,
    pub account_created: Option<DateTime<Utc>>,
    pub is_bot: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SyncMembersPayload {
    pub guild_id: String,
    pub members: Vec<MemberPayload>,
}

#[derive(Debug, Serialize)]
pub struct UpdateMemberPayload {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: Option<serde_json::Value>,
}

/// Client specifique au security-bot, encapsule le BaseApiClient partage.
pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    /// Recupere les derniers evenements de securite depuis le backend.
    pub async fn list_events(&self, guild_id: &str, limit: u32) -> Result<Vec<serde_json::Value>, String> {
        self.base
            .get_json(&format!("/api/security/events?guild_id={}&limit={}", guild_id, limit))
            .await
    }

    /// Signale un evenement de securite au backend.
    pub async fn report_event(&self, event: &SecurityEvent) -> Result<(), String> {
        self.base
            .post_fire_and_forget("/api/security/events", event)
            .await;
        Ok(())
    }

    /// Sync tous les membres d'un serveur vers l'API.
    pub async fn sync_members(&self, payload: &SyncMembersPayload) -> Result<(), String> {
        self.base
            .post_fire_and_forget("/api/members/sync", payload)
            .await;
        Ok(())
    }

    /// Enregistre un nouveau membre.
    pub async fn register_member(&self, member: &MemberPayload) -> Result<(), String> {
        self.base
            .post_fire_and_forget("/api/members/register", member)
            .await;
        Ok(())
    }

    /// Supprime un membre (depart du serveur).
    pub async fn remove_member(&self, guild_id: &str, user_id: &str) -> Result<(), String> {
        // DELETE without body — use raw client
        let req = self.base.client().delete(format!(
            "{}/api/members/{}/{}",
            self.base.base_url(), guild_id, user_id
        ));
        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("remove_member: {e}"))?;
        Ok(())
    }

    /// Met a jour un membre (changement pseudo, avatar, roles).
    pub async fn update_member(&self, guild_id: &str, user_id: &str, payload: &UpdateMemberPayload) -> Result<(), String> {
        self.base
            .patch_fire_and_forget(
                &format!("/api/members/{}/{}", guild_id, user_id),
                payload,
            )
            .await;
        Ok(())
    }
}

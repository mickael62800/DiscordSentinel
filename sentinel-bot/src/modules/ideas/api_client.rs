//! Client API du module idees.
//!
//! Le bot ne touche jamais la base : tout passe par l'API HTTP
//! (`/api/ideas`), comme les modules audit / guild_backup.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::shared::api_client::BaseApiClient;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Idea {
    pub id: String,
    pub guild_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub category: String,
    pub author_id: String,
    pub author_name: String,
    pub channel_id: Option<String>,
    pub decided_by: Option<String>,
    pub decided_by_name: Option<String>,
    pub decision_reason: Option<String>,
    pub decided_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateIdeaRequest {
    pub guild_id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub author_id: String,
    pub author_name: String,
    pub channel_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct DecideRequest<'a> {
    status: &'a str,
    decided_by: &'a str,
    decided_by_name: &'a str,
    reason: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct SetChannelRequest<'a> {
    channel_id: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct AddMessageRequest<'a> {
    author_name: &'a str,
    author_role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct QuotaResponse {
    open_count: i64,
}

pub struct ApiClient {
    base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    pub async fn create_idea(&self, req: &CreateIdeaRequest) -> Result<Idea, String> {
        self.base.post_json("/api/ideas", req).await
    }

    /// Nombre d'idees non tranchees de ce membre sur cette guild.
    pub async fn open_count(&self, guild_id: &str, author_id: &str) -> Result<i64, String> {
        let path = format!("/api/ideas/quota/{guild_id}/{author_id}");
        let resp: QuotaResponse = self.base.get_json(&path).await?;
        Ok(resp.open_count)
    }

    pub async fn idea_by_channel(&self, channel_id: &str) -> Result<Idea, String> {
        self.base
            .get_json(&format!("/api/ideas/by-channel/{channel_id}"))
            .await
    }

    pub async fn decide(
        &self,
        idea_id: &str,
        status: &str,
        decided_by: &str,
        decided_by_name: &str,
        reason: Option<&str>,
    ) -> Result<Idea, String> {
        let body = DecideRequest {
            status,
            decided_by,
            decided_by_name,
            reason,
        };
        self.base
            .patch_json(&format!("/api/ideas/{idea_id}/status"), &body)
            .await
    }

    pub async fn set_channel(&self, idea_id: &str, channel_id: Option<&str>) -> Result<(), String> {
        let body = SetChannelRequest { channel_id };
        let _: serde_json::Value = self
            .base
            .patch_json(&format!("/api/ideas/{idea_id}/channel"), &body)
            .await?;
        Ok(())
    }

    /// Sync best-effort d'un message du salon : une perte n'est pas bloquante.
    pub async fn add_message(
        &self,
        idea_id: &str,
        author_name: &str,
        author_role: &str,
        content: &str,
    ) {
        let body = AddMessageRequest {
            author_name,
            author_role,
            content,
        };
        self.base
            .post_fire_and_forget(&format!("/api/ideas/{idea_id}/messages"), &body)
            .await;
    }
}

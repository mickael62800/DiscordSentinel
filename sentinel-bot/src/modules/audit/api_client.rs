use std::sync::Arc;

use crate::shared::api_client::BaseApiClient;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub details: serde_json::Value,
}

pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    pub async fn search_audit_logs(
        &self,
        guild_id: &str,
        target_id: Option<&str>,
        event_type: Option<&str>,
        limit: u32,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut path = format!("/api/audit-logs?guild_id={}&limit={}", guild_id, limit);
        if let Some(tid) = target_id {
            path.push_str(&format!("&target_id={}", tid));
        }
        if let Some(et) = event_type {
            path.push_str(&format!("&event_type={}", et));
        }

        self.base.get_json(&path).await
    }

    /// Recupere les IDs de tous les utilisateurs surveilles (batch, tous les serveurs).
    /// Un seul appel API au lieu de N appels par guild.
    pub async fn get_all_watched_user_ids(&self) -> Result<Vec<String>, String> {
        let users: Vec<serde_json::Value> =
            self.base.get_json("/api/watched-users?limit=1000").await?;

        Ok(users
            .iter()
            .filter_map(|u| {
                u.get("user_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect())
    }

    /// Enregistre un evenement d'activite pour un utilisateur surveille
    pub async fn log_user_activity(
        &self,
        guild_id: &str,
        user_id: &str,
        event_type: &str,
        channel_id: Option<&str>,
        channel_name: Option<&str>,
        content: Option<&str>,
        metadata: serde_json::Value,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "guild_id": guild_id,
            "user_id": user_id,
            "event_type": event_type,
            "channel_id": channel_id,
            "channel_name": channel_name,
            "content": content,
            "metadata": metadata,
        });

        self.base
            .post_fire_and_forget("/api/user-activity", &payload)
            .await;
        Ok(())
    }

    pub async fn send_audit_event(&self, event: &AuditEvent) -> Result<(), String> {
        let _: serde_json::Value = self.base.post_json("/api/audit-logs", event).await?;
        Ok(())
    }
}

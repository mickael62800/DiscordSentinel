use std::sync::Arc;

use serde::Serialize;
use sentinel_shared::api_client::BaseApiClient;

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
        let mut url = format!(
            "{}/api/audit-logs?guild_id={}&limit={}",
            self.base.base_url(),
            guild_id,
            limit
        );
        if let Some(tid) = target_id {
            url.push_str(&format!("&target_id={}", tid));
        }
        if let Some(et) = event_type {
            url.push_str(&format!("&event_type={}", et));
        }

        let req = self.base.client().get(&url);

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("{e}"))?
            .json()
            .await
            .map_err(|e| format!("{e}"))
    }

    /// Recupere les IDs des utilisateurs surveilles pour un serveur
    pub async fn get_watched_user_ids(&self, guild_id: &str) -> Result<Vec<String>, String> {
        let url = format!(
            "{}/api/watched-users?guild_id={}",
            self.base.base_url(),
            guild_id
        );

        let req = self.base.client().get(&url);
        let users: Vec<serde_json::Value> = self
            .base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("{e}"))?
            .json()
            .await
            .map_err(|e| format!("{e}"))?;

        Ok(users
            .iter()
            .filter_map(|u| u.get("user_id").and_then(|v| v.as_str()).map(|s| s.to_string()))
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
        let req = self
            .base
            .client()
            .post(format!("{}/api/user-activity", self.base.base_url()))
            .json(&serde_json::json!({
                "guild_id": guild_id,
                "user_id": user_id,
                "event_type": event_type,
                "channel_id": channel_id,
                "channel_name": channel_name,
                "content": content,
                "metadata": metadata,
            }));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    pub async fn send_audit_event(&self, event: &AuditEvent) -> Result<(), String> {
        let req = self
            .base
            .client()
            .post(format!("{}/api/audit-logs", self.base.base_url()))
            .json(event);

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }
}

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

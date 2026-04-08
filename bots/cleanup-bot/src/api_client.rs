use std::sync::Arc;

use serde::Deserialize;

use sentinel_shared::api_client::BaseApiClient;

#[derive(Debug, Deserialize)]
pub struct PurgeResponse {
    #[serde(default)]
    pub deleted: u64,
}

pub struct ApiClient {
    pub base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    pub async fn purge_infractions(&self, guild_id: &str, days: u64) -> Result<u64, String> {
        let resp: PurgeResponse = self.base.delete_with_body(
            "/api/purge/infractions",
            &serde_json::json!({ "guild_id": guild_id, "days": days }),
        ).await?;
        Ok(resp.deleted)
    }

    pub async fn purge_audit_logs(&self, guild_id: &str, days: u64) -> Result<u64, String> {
        let resp: PurgeResponse = self.base.delete_with_body(
            "/api/purge/audit-logs",
            &serde_json::json!({ "guild_id": guild_id, "days": days }),
        ).await?;
        Ok(resp.deleted)
    }

    pub async fn purge_logs(&self, days: u64) -> Result<u64, String> {
        let resp: PurgeResponse = self.base.delete_with_body(
            "/api/purge/logs",
            &serde_json::json!({ "days": days }),
        ).await?;
        Ok(resp.deleted)
    }
}

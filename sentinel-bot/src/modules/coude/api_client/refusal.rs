//! API client refusals/dette d honneur (cf. COUPE_AMELIORATIONS 5.3).

use serde::Deserialize;

use super::ApiClient;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RefusalCountResp {
    pub count: i32,
    pub honor_debt_owed: bool,
}

impl ApiClient {
    /// Increment best-effort : si l API casse on log mais on ne casse
    /// pas le flow /refuser.
    pub async fn increment_refusal(&self, guild_id: &str, requester_id: &str, refuser_id: &str) {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/refusals/{requester_id}/{refuser_id}/increment"),
                &serde_json::json!({}),
            )
            .await;
    }

    pub async fn get_refusal_count(
        &self,
        guild_id: &str,
        requester_id: &str,
        refuser_id: &str,
    ) -> Result<RefusalCountResp, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/refusals/{requester_id}/{refuser_id}"
            ))
            .await
    }

    pub async fn reset_refusal(&self, guild_id: &str, requester_id: &str, refuser_id: &str) {
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/refusals/{requester_id}/{refuser_id}/reset"),
                &serde_json::json!({}),
            )
            .await;
    }
}

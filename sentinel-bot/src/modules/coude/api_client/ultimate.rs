//! API client ultimates (cf. COUPE_AMELIORATIONS 3.1).

use serde::{Deserialize, Serialize};

use super::ApiClient;

#[derive(Debug, Serialize)]
pub struct ActivateUltimateBody<'a> {
    pub kind: &'a str,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct UltimateStateResp {
    pub pending_kind: Option<String>,
    pub last_used_at: Option<String>,
    pub activated_at: Option<String>,
}

impl ApiClient {
    pub async fn activate_ultimate(
        &self,
        guild_id: &str,
        user_id: &str,
        kind: &str,
    ) -> Result<UltimateStateResp, String> {
        let body = ActivateUltimateBody { kind };
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/ultimates/{user_id}/activate"),
                &body,
            )
            .await
    }

    pub async fn get_ultimate_state(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UltimateStateResp, String> {
        self.base
            .get_json(&format!("/api/coude/{guild_id}/ultimates/{user_id}"))
            .await
    }
}

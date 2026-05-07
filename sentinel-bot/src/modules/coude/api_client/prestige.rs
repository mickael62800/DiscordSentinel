//! API client prestige (cf. COUPE_AMELIORATIONS 3.3).

use serde::Deserialize;

use super::ApiClient;

#[derive(Debug, Deserialize, Clone)]
pub struct PrestigeOutcomeResp {
    pub new_prestige_count: i32,
    pub stars: String,
}

impl ApiClient {
    pub async fn prestige_player(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<PrestigeOutcomeResp, String> {
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/players/{user_id}/prestige"),
                &serde_json::json!({}),
            )
            .await
    }
}

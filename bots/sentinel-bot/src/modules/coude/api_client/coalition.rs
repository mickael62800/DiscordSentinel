//! API client coalitions (cf. COUPE_AMELIORATIONS 5.3).

use serde::{Deserialize, Serialize};

use super::ApiClient;
use sentinel_api::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Deserialize, Clone)]
pub struct CoalitionMemberResp {
    pub member_id: String,
    pub member_name: String,
    pub joined_at: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ActiveCoalitionResp {
    pub id: String,
    pub guild_id: GuildId,
    pub target_id: String,
    pub opened_at: String,
    pub expires_at: String,
    pub status: String,
    pub broken_by: Option<String>,
    pub broken_at: Option<String>,
    pub members: Vec<CoalitionMemberResp>,
}

#[derive(Debug, Serialize)]
pub struct JoinCoalitionBody<'a> {
    pub target_id: &'a str,
    pub member_id: &'a str,
    pub member_name: &'a str,
}

impl ApiClient {
    pub async fn join_coalition(
        &self,
        guild_id: &str,
        target_id: &str,
        member_id: &str,
        member_name: &str,
    ) -> Result<ActiveCoalitionResp, String> {
        let body = JoinCoalitionBody {
            target_id,
            member_id,
            member_name,
        };
        self.base
            .post_json(&format!("/api/coude/{guild_id}/coalitions/join"), &body)
            .await
    }

    pub async fn get_coalition_by_target(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveCoalitionResp>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/coalitions/by-target/{target_id}"
            ))
            .await
    }
}

//! API client primes collectives (cf. COUPE_AMELIORATIONS 5.3).

use serde::{Deserialize, Serialize};

use super::ApiClient;
use sentinel_api::domain::entities::system::discord_ids::GuildId;

#[derive(Debug, Deserialize, Clone)]
pub struct ActiveBountyResp {
    pub id: String,
    pub guild_id: GuildId,
    pub target_id: String,
    pub total_amount: i64,
    pub status: String,
    pub opened_at: String,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContributeBountyBody<'a> {
    pub contributor_id: &'a str,
    pub contributor_name: &'a str,
    pub amount: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContributedBountyResp {
    pub bounty_id: String,
    pub new_total: i64,
}

impl ApiClient {
    pub async fn get_bounty_by_target(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<Option<ActiveBountyResp>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/bounties/by-target/{target_id}"
            ))
            .await
    }

    pub async fn contribute_to_bounty(
        &self,
        guild_id: &str,
        target_id: &str,
        contributor_id: &str,
        contributor_name: &str,
        amount: i64,
    ) -> Result<ContributedBountyResp, String> {
        let body = ContributeBountyBody {
            contributor_id,
            contributor_name,
            amount,
        };
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/bounties/by-target/{target_id}/contribute"),
                &body,
            )
            .await
    }
}

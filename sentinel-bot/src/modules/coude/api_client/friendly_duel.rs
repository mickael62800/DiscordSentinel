//! API client duel amical (cf. COUPE_AMELIORATIONS 4.5).

use serde::{Deserialize, Serialize};

use super::ApiClient;

#[derive(Debug, Serialize)]
struct FriendlyDuelBody<'a> {
    attacker_id: &'a str,
    attacker_name: &'a str,
    defender_id: &'a str,
    defender_name: &'a str,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FriendlyDuelResp {
    pub winner_id: Option<String>,
    pub loser_id: Option<String>,
    pub draw: bool,
    pub total_rounds: i32,
    pub attacker_hp_final: i32,
    pub attacker_hp_max: i32,
    pub defender_hp_final: i32,
    pub defender_hp_max: i32,
    pub winner_xp: i64,
    pub loser_xp: i64,
}

impl ApiClient {
    pub async fn resolve_friendly_duel(
        &self,
        guild_id: &str,
        attacker_id: &str,
        attacker_name: &str,
        defender_id: &str,
        defender_name: &str,
    ) -> Result<FriendlyDuelResp, String> {
        let body = FriendlyDuelBody {
            attacker_id,
            attacker_name,
            defender_id,
            defender_name,
        };
        self.base
            .post_json(&format!("/api/coude/{guild_id}/friendly-duels"), &body)
            .await
    }
}

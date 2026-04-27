//! API client steal_roll (Phase 2 #4 audit).
//!
//! Le bot delegue le tirage RNG (d20 thief/victim + % wallet) au serveur
//! pour rendre la decision auditable. La presentation (templates,
//! embeds, calcul des bonus class/DEF/boost, application du verdict via
//! record_steal) reste cote bot.

use serde::{Deserialize, Serialize};

use super::ApiClient;

#[derive(Debug, Serialize)]
pub struct RollStealBody {
    pub afk: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RollStealResp {
    pub thief_d20: i32,
    pub victim_d20: i32,
    /// Basis points (1bp = 0.01%). Diviser par 10_000 pour obtenir le
    /// ratio applicable au solde de la victime.
    pub steal_pct_bp: u32,
}

impl ApiClient {
    pub async fn roll_steal(
        &self,
        guild_id: &str,
        afk: bool,
    ) -> Result<RollStealResp, String> {
        let body = RollStealBody { afk };
        self.base
            .post_json(&format!("/api/coude/{guild_id}/steal/roll"), &body)
            .await
    }
}

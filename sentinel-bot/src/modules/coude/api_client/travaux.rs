//! API client /travaux (Phase 2 #2 audit).
//!
//! Le RNG est cote API. Le bot ne fait qu'envoyer la commande et afficher
//! le verdict (task choisie, succes/echec, coins, flavor).

use serde::{Deserialize, Serialize};

use super::ApiClient;

#[derive(Debug, Serialize)]
pub struct PlayTravauxBody<'a> {
    pub user_id: &'a str,
    pub username: &'a str,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlayTravauxResp {
    pub task_key: String,
    pub task_label: String,
    pub task_description: String,
    pub success: bool,
    pub flavor: String,
    pub coins_gain: i64,
    pub xp_gain: i64,
}

impl ApiClient {
    pub async fn play_travaux(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<PlayTravauxResp, String> {
        let body = PlayTravauxBody { user_id, username };
        self.base
            .post_json(&format!("/api/coude/{guild_id}/travaux/play"), &body)
            .await
    }
}

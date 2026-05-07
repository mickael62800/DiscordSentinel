//! API client tout-ou-rien (record + memorial). Cf. COUPE_AMELIORATIONS 6.1.

use serde::{Deserialize, Serialize};

use super::ApiClient;

#[derive(Debug, Serialize)]
pub struct RecordToutOuRienBody<'a> {
    pub user_id: &'a str,
    pub username: &'a str,
    pub mise: i64,
    pub outcome: &'a str, // "won" | "lost"
    pub delta: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MemorialEntryResp {
    pub user_id: String,
    pub username: String,
    pub mise: i64,
    pub outcome: String,
    pub delta: i64,
    pub created_at: String,
}

impl ApiClient {
    /// Loggue une tentative tout-ou-rien (fire-and-forget : si l API
    /// indispo on ne casse pas la commande utilisateur).
    pub async fn record_tout_ou_rien(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        mise: i64,
        outcome: &str,
        delta: i64,
    ) {
        let body = RecordToutOuRienBody {
            user_id,
            username,
            mise,
            outcome,
            delta,
        };
        self.base
            .post_fire_and_forget(
                &format!("/api/coude/{guild_id}/tout-ou-rien/record"),
                &body,
            )
            .await;
    }

    pub async fn get_memorial(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<MemorialEntryResp>, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/tout-ou-rien/memorial?limit={limit}"
            ))
            .await
    }

    pub async fn get_user_tout_ou_rien_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<ToutOuRienUserStatsResp, String> {
        self.base
            .get_json(&format!(
                "/api/coude/{guild_id}/tout-ou-rien/by-user/{user_id}"
            ))
            .await
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ToutOuRienUserStatsResp {
    pub attempts: i64,
    pub wins: i64,
    pub losses: i64,
    pub biggest_win: i64,
    pub biggest_loss: i64,
}

// ── Phase 2 #1 audit : RNG migre cote API. Le bot appelle un seul
//    endpoint `play` qui fait cooldown + verif solde + RNG + mutation
//    wallet + cooldown + memorial. ────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PlayToutOuRienBody<'a> {
    pub user_id: &'a str,
    pub username: &'a str,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlayToutOuRienResp {
    pub initial_coins: i64,
    /// "won" | "lost"
    pub outcome: String,
    pub delta: i64,
    pub final_balance: i64,
}

impl ApiClient {
    /// Joue un tout-ou-rien : pipeline complet cote API.
    /// Renvoie une erreur (RateLimited / ValidationError / Internal) que
    /// le bot affiche tel quel a l'utilisateur.
    pub async fn play_tout_ou_rien(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<PlayToutOuRienResp, String> {
        let body = PlayToutOuRienBody { user_id, username };
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/tout-ou-rien/play"),
                &body,
            )
            .await
    }
}

//! API client /prank (Phase 3 finalisation audit).

use serde::Deserialize;

use super::ApiClient;

#[derive(Debug, Deserialize, Clone)]
pub struct PrankBraquageRollResp {
    pub amount: i64,
}

impl ApiClient {
    /// Tirage cote API du faux montant pour `/prank type:braquage`.
    /// Statless 5_000..=50_000 par pas de 1 000.
    pub async fn roll_prank_braquage_amount(
        &self,
        guild_id: &str,
    ) -> Result<PrankBraquageRollResp, String> {
        // Body vide — tonic-style accepte aussi `()` mais reqwest a besoin
        // d'un body JSON valide. On envoie `{}`.
        self.base
            .post_json(
                &format!("/api/coude/{guild_id}/prank/braquage/roll"),
                &serde_json::json!({}),
            )
            .await
    }
}

use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;
use crate::domain::entities::system::discord_ids::ChannelId;

#[derive(Debug, Serialize)]
pub struct WheelSpinRequest {
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TauntEventDto {
    pub channel_id: ChannelId,
    pub target_user_id: String,
    pub message: String,
    pub nickname_suffix: String,
    pub streak_kind: String,
    pub streak_value: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WheelSpinResponse {
    #[allow(dead_code)]
    pub spin_id: String,
    /// Cle stable de la case tiree (jackpot/licorne/blanche/...). Utilise
    /// par les tests + futur affichage stats par case.
    #[allow(dead_code)]
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub balance_after: i64,
    pub is_memorable: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub triggered_taunts: Vec<TauntEventDto>,
}

pub async fn spin(
    client: &BaseApiClient,
    guild_id: &str,
    req: &WheelSpinRequest,
) -> Result<WheelSpinResponse, String> {
    client.post_json(&format!("/api/wheel/{guild_id}/spin"), req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jackpot_response() {
        let json = r#"{
            "spin_id": "00000000-0000-0000-0000-000000000000",
            "case_key": "jackpot",
            "case_label": "🎰 Jackpot — +5000c",
            "payout": 5000,
            "balance_after": 6000,
            "is_memorable": true
        }"#;
        let r: WheelSpinResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.case_key, "jackpot");
        assert_eq!(r.payout, 5000);
        assert!(r.is_memorable);
        assert!(r.triggered_taunts.is_empty());
    }

    #[test]
    fn parses_loss_response() {
        let json = r#"{
            "spin_id": "00000000-0000-0000-0000-000000000000",
            "case_key": "ruine",
            "case_label": "💀 Ruine",
            "payout": -500,
            "balance_after": 100,
            "is_memorable": false
        }"#;
        let r: WheelSpinResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.payout, -500);
        assert!(!r.is_memorable);
    }

    #[test]
    fn parses_blanche_neutral() {
        let json = r#"{
            "spin_id": "x",
            "case_key": "blanche",
            "case_label": "🌀 Blanche",
            "payout": 0,
            "balance_after": 500,
            "is_memorable": false
        }"#;
        let r: WheelSpinResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.payout, 0);
    }
}

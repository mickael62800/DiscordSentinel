//! Client API HTTP du module slot. Wrapper autour de `BaseApiClient`.

use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;

#[derive(Debug, Serialize)]
pub struct SpinRequest {
    pub user_id: String,
    pub username: String,
    pub mise: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyRequest {
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct TauntEventDto {
    pub channel_id: String,
    pub target_user_id: String,
    pub message: String,
    pub nickname_suffix: String,
    pub streak_kind: String,
    pub streak_value: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpinResponse {
    #[allow(dead_code)]
    pub spin_id: String,
    pub symbols: Vec<String>,
    pub mise: i64,
    pub payout: i64,
    pub multiplier: f64,
    pub is_jackpot: bool,
    pub is_free: bool,
    pub jackpot_pool_after: i64,
    pub balance_after: i64,
    /// Taunts a rejouer (faillite, jackpot eco). Pas encore consume cote
    /// bot — placeholder pour quand on integrera le replay des taunts.
    #[serde(default)]
    #[allow(dead_code)]
    pub triggered_taunts: Vec<TauntEventDto>,
}

/// Reponse `GET /api/slot/{guild}/jackpot` — pas encore consommee cote bot
/// (panel n affiche pas le pool en live), utilisee par les tests + futur
/// affichage stats.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct JackpotPoolResponse {
    pub current_pool: i64,
}

pub async fn spin(client: &BaseApiClient, guild_id: &str, req: &SpinRequest) -> Result<SpinResponse, String> {
    client.post_json(&format!("/api/slot/{guild_id}/spin"), req).await
}

pub async fn daily(client: &BaseApiClient, guild_id: &str, req: &DailyRequest) -> Result<SpinResponse, String> {
    client.post_json(&format!("/api/slot/{guild_id}/daily"), req).await
}

#[allow(dead_code)]
pub async fn jackpot_pool(client: &BaseApiClient, guild_id: &str) -> Result<i64, String> {
    let r: JackpotPoolResponse = client.get_json(&format!("/api/slot/{guild_id}/jackpot")).await?;
    Ok(r.current_pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_spin_response() {
        let json = r#"{
            "spin_id": "00000000-0000-0000-0000-000000000000",
            "symbols": ["🍒","🍒","🍋"],
            "mise": 50,
            "payout": 50,
            "multiplier": 1.0,
            "is_jackpot": false,
            "is_free": false,
            "jackpot_pool_after": 1000,
            "balance_after": 200
        }"#;
        let r: SpinResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.symbols.len(), 3);
        assert_eq!(r.mise, 50);
        assert!(!r.is_jackpot);
        assert!(r.triggered_taunts.is_empty());
    }

    #[test]
    fn parses_jackpot_response_with_taunts() {
        let json = r#"{
            "spin_id": "00000000-0000-0000-0000-000000000000",
            "symbols": ["7️⃣","7️⃣","7️⃣"],
            "mise": 100,
            "payout": 60000,
            "multiplier": 100.0,
            "is_jackpot": true,
            "is_free": false,
            "jackpot_pool_after": 1000,
            "balance_after": 60100,
            "triggered_taunts": [{
                "channel_id": "12345",
                "target_user_id": "67890",
                "message": "JACKPOT !",
                "nickname_suffix": " | Jackpot",
                "streak_kind": "eco_jackpot",
                "streak_value": 1
            }]
        }"#;
        let r: SpinResponse = serde_json::from_str(json).unwrap();
        assert!(r.is_jackpot);
        assert_eq!(r.payout, 60000);
        assert_eq!(r.triggered_taunts.len(), 1);
        assert_eq!(r.triggered_taunts[0].streak_kind, "eco_jackpot");
    }

    #[test]
    fn parses_jackpot_pool_response() {
        let json = r#"{"current_pool": 12345}"#;
        let r: JackpotPoolResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.current_pool, 12345);
    }
}

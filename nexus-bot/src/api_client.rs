//! Client HTTP minimal vers nexus-api (modele BaseApiClient simplifie).

use serde::Deserialize;
use serde::Serialize;

pub struct ApiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WheelSpinRequest {
    pub username: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WheelSpinResponse {
    #[allow(dead_code)]
    pub spin_id: String,
    #[allow(dead_code)]
    pub case_key: String,
    pub case_label: String,
    pub payout: i64,
    pub balance_after: i64,
    pub is_memorable: bool,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalletResponse {
    #[allow(dead_code)]
    pub guild_id: String,
    pub user_id: String,
    #[allow(dead_code)]
    pub username: String,
    pub coins: i64,
    pub total_earned: i64,
    pub total_spent: i64,
}

#[derive(Debug, Serialize)]
pub struct TransferRequest {
    pub from_user_id: String,
    pub from_username: String,
    pub to_user_id: String,
    pub to_username: String,
    pub amount: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransferResponse {
    pub amount: i64,
    pub from_balance: i64,
    #[allow(dead_code)]
    pub to_balance: i64,
}

impl ApiClient {
    /// `base_url` ex. http://nexus-api:3100 (NEXUS_API_URL).
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    /// Envoie la requete, mappe 4xx/5xx vers un message affichable.
    async fn send<T: serde::de::DeserializeOwned>(
        &self,
        mut req: reqwest::RequestBuilder,
    ) -> Result<T, String> {
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("nexus-api injoignable: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            resp.json::<T>()
                .await
                .map_err(|e| format!("reponse nexus-api invalide: {e}"))
        } else {
            Err(resp
                .json::<ApiErrorBody>()
                .await
                .map(|b| b.error)
                .unwrap_or_else(|_| format!("erreur nexus-api ({status})")))
        }
    }

    /// GET /api/wallet/{guild_id}/{user_id}.
    pub async fn get_wallet(&self, guild_id: &str, user_id: &str) -> Result<WalletResponse, String> {
        let url = format!("{}/api/wallet/{guild_id}/{user_id}", self.base_url);
        self.send(self.http.get(&url)).await
    }

    /// POST /api/wallet/{guild_id}/transfer.
    pub async fn transfer_coins(
        &self,
        guild_id: &str,
        req: &TransferRequest,
    ) -> Result<TransferResponse, String> {
        let url = format!("{}/api/wallet/{guild_id}/transfer", self.base_url);
        self.send(self.http.post(&url).json(req)).await
    }

    /// GET /api/wallet/{guild_id}/leaderboard?limit=N.
    pub async fn wallet_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<WalletResponse>, String> {
        let url = format!(
            "{}/api/wallet/{guild_id}/leaderboard?limit={limit}",
            self.base_url
        );
        self.send(self.http.get(&url)).await
    }

    /// POST /api/wheel/{guild_id}/{user_id}/spin.
    /// Err(message affichable) sur 4xx (ex: daily deja claim) ou erreur reseau.
    pub async fn spin_wheel(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
    ) -> Result<WheelSpinResponse, String> {
        let url = format!("{}/api/wheel/{guild_id}/{user_id}/spin", self.base_url);
        let mut req = self.http.post(&url).json(&WheelSpinRequest {
            username: username.to_string(),
        });
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("nexus-api injoignable: {e}"))?;

        let status = resp.status();
        if status.is_success() {
            resp.json::<WheelSpinResponse>()
                .await
                .map_err(|e| format!("reponse nexus-api invalide: {e}"))
        } else {
            let msg = resp
                .json::<ApiErrorBody>()
                .await
                .map(|b| b.error)
                .unwrap_or_else(|_| format!("erreur nexus-api ({status})"));
            Err(msg)
        }
    }
}

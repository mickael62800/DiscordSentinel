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

impl ApiClient {
    /// `base_url` ex. http://nexus-api:3100 (NEXUS_API_URL).
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
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

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

/// Action de modération envoyée au backend.
#[derive(Debug, Serialize)]
pub struct ModerationAction {
    pub guild_id: String,
    pub channel_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    /// Gravité pour les warns : "low", "medium", "high"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gravity: Option<String>,
    /// Durée en secondes (None = permanent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ModerationActionResponse {
    pub id: String,
    pub action_type: String,
    pub target_name: String,
    pub reason: String,
}

/// Historique des sanctions d'un utilisateur.
#[derive(Debug, Deserialize)]
pub struct UserHistory {
    pub target_id: String,
    pub target_name: String,
    pub total_warns: u32,
    pub total_mutes: u32,
    pub total_bans: u32,
    pub actions: Vec<ModerationActionResponse>,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: config.api_base_url.clone(),
            api_key: config.api_key.clone(),
        }
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.api_key.is_empty() {
            req
        } else {
            req.bearer_auth(&self.api_key)
        }
    }

    /// Enregistre une action de modération dans le backend.
    pub async fn log_action(&self, action: &ModerationAction) -> Result<ModerationActionResponse, String> {
        let req = self
            .client
            .post(format!("{}/api/moderation/actions", self.base_url))
            .json(action);

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<ModerationActionResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Récupère l'historique des sanctions d'un utilisateur.
    pub async fn get_history(&self, guild_id: &str, user_id: &str) -> Result<UserHistory, String> {
        let req = self
            .client
            .get(format!(
                "{}/api/moderation/history/{}/{}",
                self.base_url, guild_id, user_id
            ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<UserHistory>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }
}

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

// ── Response DTOs ──

#[derive(Debug, Deserialize)]
pub struct Infraction {
    pub id: String,
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub action: String,
    pub reason: Option<String>,
    pub score: f64,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UserStatsResponse {
    pub guild_id: String,
    pub user_id: String,
    pub username: String,
    pub message_count: u64,
    pub voice_seconds: u64,
    pub voice_hours: f64,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GuildOverviewResponse {
    pub guild_id: String,
    pub total_messages: u64,
    pub total_voice_seconds: u64,
    pub total_voice_hours: f64,
    pub active_members: u64,
    pub total_infractions: u64,
    pub total_warns: u64,
    pub total_mutes: u64,
    pub total_bans: u64,
    pub top_members: Vec<UserStatsResponse>,
}

// ── Request DTOs ──

#[derive(Debug, Serialize)]
struct RecordMessagesPayload {
    guild_id: String,
    user_id: String,
    username: String,
    count: u64,
}

#[derive(Debug, Serialize)]
struct RecordVoicePayload {
    guild_id: String,
    user_id: String,
    username: String,
    seconds: u64,
}

// ── Client ──

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

    /// Envoie un batch de messages comptés au backend.
    pub async fn record_messages(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        count: u64,
    ) -> Result<(), String> {
        let req = self
            .client
            .post(format!("{}/api/stats/messages", self.base_url))
            .json(&RecordMessagesPayload {
                guild_id: guild_id.to_string(),
                user_id: user_id.to_string(),
                username: username.to_string(),
                count,
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?;

        Ok(())
    }

    /// Envoie le temps vocal passé au backend.
    pub async fn record_voice(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        seconds: u64,
    ) -> Result<(), String> {
        let req = self
            .client
            .post(format!("{}/api/stats/voice", self.base_url))
            .json(&RecordVoicePayload {
                guild_id: guild_id.to_string(),
                user_id: user_id.to_string(),
                username: username.to_string(),
                seconds,
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?;

        Ok(())
    }

    /// Récupère les stats d'un utilisateur depuis le backend.
    pub async fn get_user_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserStatsResponse>, String> {
        let req = self.client.get(format!(
            "{}/api/stats/{guild_id}/user/{user_id}",
            self.base_url
        ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<Option<UserStatsResponse>>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Récupère les stats globales du serveur.
    pub async fn get_guild_overview(
        &self,
        guild_id: &str,
    ) -> Result<GuildOverviewResponse, String> {
        let req = self.client.get(format!(
            "{}/api/stats/{guild_id}/overview",
            self.base_url
        ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<GuildOverviewResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Récupère le classement des membres.
    pub async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserStatsResponse>, String> {
        let req = self.client.get(format!(
            "{}/api/stats/{guild_id}/leaderboard?limit={limit}",
            self.base_url
        ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<Vec<UserStatsResponse>>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Récupère les infractions d'un serveur.
    pub async fn get_infractions(&self, guild_id: &str) -> Result<Vec<Infraction>, String> {
        let req = self
            .client
            .get(format!("{}/infractions/{guild_id}", self.base_url));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<Vec<Infraction>>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }
}

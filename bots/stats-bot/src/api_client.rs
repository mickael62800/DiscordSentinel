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

#[derive(Debug, Deserialize)]
pub struct UserLevelResponse {
    pub user_id: String,
    pub username: String,
    pub xp: i64,
    pub level: i32,
    pub xp_current: i64,
    pub xp_needed: i64,
}

#[derive(Debug, Deserialize)]
pub struct AddXpResponse {
    pub user: UserLevelResponse,
    pub leveled_up: bool,
    pub old_level: i32,
    pub reward_role_id: Option<String>,
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
    const BOT_NAME: &'static str = "stats-bot";

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

    pub fn send_log(&self, level: &str, server: &str, message: &str) {
        #[derive(Serialize)]
        struct LogPayload { level: String, bot: String, server: String, message: String, category: String }
        let req = self.auth(self.client.post(format!("{}/api/logs", self.base_url))
            .json(&LogPayload {
                level: level.to_string(),
                bot: Self::BOT_NAME.to_string(),
                server: server.to_string(),
                message: message.to_string(),
                category: "bot".to_string(),
            }));
        tokio::spawn(async move { let _ = req.send().await; });
    }

    pub async fn heartbeat(&self, name: &str) -> Result<(), String> {
        #[derive(serde::Serialize)]
        struct Payload { name: String }

        let mut req = self.client
            .post(format!("{}/api/bots/heartbeat", self.base_url))
            .json(&Payload { name: name.to_string() });

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send().await.map_err(|e| format!("Heartbeat failed: {e}"))?;
        Ok(())
    }

    pub async fn get_guild_config(&self, guild_id: &str) -> Result<std::collections::HashMap<String, String>, String> {
        let url = format!("{}/api/bots/config/{}/{}", self.base_url, guild_id, Self::BOT_NAME);
        let mut req = self.client.get(&url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        #[derive(serde::Deserialize)]
        struct ConfigEntry {
            config_key: String,
            config_value: String,
        }

        let resp = req.send().await.map_err(|e| format!("Config fetch failed: {e}"))?;
        let entries: Vec<ConfigEntry> = resp.json().await.map_err(|e| format!("Config parse failed: {e}"))?;
        Ok(entries.into_iter().map(|e| (e.config_key, e.config_value)).collect())
    }

    /// Helper pour lire une valeur de config avec fallback
    pub fn config_or(config: &std::collections::HashMap<String, String>, key: &str, default: &str) -> String {
        config.get(key).cloned().unwrap_or_else(|| default.to_string())
    }

    pub fn config_u64(config: &std::collections::HashMap<String, String>, key: &str, default: u64) -> u64 {
        config.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
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

    // ── Levels / XP ──

    pub async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        amount: i64,
    ) -> Result<AddXpResponse, String> {
        #[derive(Serialize)]
        struct Payload {
            guild_id: String,
            user_id: String,
            username: String,
            amount: i64,
        }

        let req = self
            .client
            .post(format!("{}/api/levels/xp", self.base_url))
            .json(&Payload {
                guild_id: guild_id.to_string(),
                user_id: user_id.to_string(),
                username: username.to_string(),
                amount,
            });

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<AddXpResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserLevelResponse>, String> {
        let req = self.client.get(format!(
            "{}/api/levels/{guild_id}/{user_id}",
            self.base_url
        ));

        let resp = self.auth(req).send().await.map_err(|e| format!("Erreur réseau: {e}"))?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        resp.json::<UserLevelResponse>()
            .await
            .map(Some)
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn get_level_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserLevelResponse>, String> {
        let req = self.client.get(format!(
            "{}/api/levels/{guild_id}/leaderboard?limit={limit}",
            self.base_url
        ));

        self.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur réseau: {e}"))?
            .json::<Vec<UserLevelResponse>>()
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

    pub async fn register_guild(&self, guild_id: &str, name: &str, member_count: i32) -> Result<(), String> {
        #[derive(serde::Serialize)]
        struct Payload {
            guild_id: String,
            name: String,
            member_count: Option<i32>,
        }

        let mut req = self.client
            .post(format!("{}/api/guilds/register", self.base_url))
            .json(&Payload {
                guild_id: guild_id.to_string(),
                name: name.to_string(),
                member_count: Some(member_count),
            });

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        req.send().await.map_err(|e| format!("Guild register failed: {e}"))?;
        Ok(())
    }
}

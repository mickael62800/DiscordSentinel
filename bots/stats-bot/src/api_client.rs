use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sentinel_shared::api_client::BaseApiClient;

// ── Response DTOs ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct UserLevelResponse {
    pub user_id: String,
    pub username: String,
    pub xp: i64,
    pub level: i32,
    pub xp_current: i64,
    pub xp_needed: i64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
    channel_id: String,
    channel_name: String,
}

// ── Client ──

/// Client API specifique au stats-bot. Delegue les appels generiques au BaseApiClient.
pub struct ApiClient {
    base: Arc<BaseApiClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>) -> Self {
        Self { base }
    }

    /// Envoie un batch de messages comptes au backend.
    pub async fn record_messages(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        count: u64,
    ) -> Result<(), String> {
        let req = self
            .base
            .client()
            .post(format!("{}/api/stats/messages", self.base.base_url()))
            .json(&RecordMessagesPayload {
                guild_id: guild_id.to_string(),
                user_id: user_id.to_string(),
                username: username.to_string(),
                count,
            });

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    /// Envoie le temps vocal passe au backend.
    pub async fn record_voice(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        seconds: u64,
        channel_id: &str,
        channel_name: &str,
    ) -> Result<(), String> {
        let req = self
            .base
            .client()
            .post(format!("{}/api/stats/voice", self.base.base_url()))
            .json(&RecordVoicePayload {
                guild_id: guild_id.to_string(),
                user_id: user_id.to_string(),
                username: username.to_string(),
                seconds,
                channel_id: channel_id.to_string(),
                channel_name: channel_name.to_string(),
            });

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?;

        Ok(())
    }

    /// Recupere les stats d'un utilisateur depuis le backend.
    pub async fn get_user_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserStatsResponse>, String> {
        let req = self.base.client().get(format!(
            "{}/api/stats/{guild_id}/user/{user_id}",
            self.base.base_url()
        ));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<Option<UserStatsResponse>>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Recupere les stats globales du serveur.
    pub async fn get_guild_overview(
        &self,
        guild_id: &str,
    ) -> Result<GuildOverviewResponse, String> {
        let req = self.base.client().get(format!(
            "{}/api/stats/{guild_id}/overview",
            self.base.base_url()
        ));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<GuildOverviewResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Recupere le classement des membres.
    pub async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserStatsResponse>, String> {
        let req = self.base.client().get(format!(
            "{}/api/stats/{guild_id}/leaderboard?limit={limit}",
            self.base.base_url()
        ));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
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
            .base
            .client()
            .post(format!("{}/api/levels/xp", self.base.base_url()))
            .json(&Payload {
                guild_id: guild_id.to_string(),
                user_id: user_id.to_string(),
                username: username.to_string(),
                amount,
            });

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<AddXpResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    pub async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserLevelResponse>, String> {
        let req = self.base.client().get(format!(
            "{}/api/levels/{guild_id}/{user_id}",
            self.base.base_url()
        ));

        let resp = self.base.auth(req).send().await.map_err(|e| format!("Erreur reseau: {e}"))?;
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
        let req = self.base.client().get(format!(
            "{}/api/levels/{guild_id}/leaderboard?limit={limit}",
            self.base.base_url()
        ));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<Vec<UserLevelResponse>>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Recupere les infractions d'un serveur.
    pub async fn get_infractions(&self, guild_id: &str) -> Result<Vec<Infraction>, String> {
        let req = self
            .base
            .client()
            .get(format!("{}/infractions/{guild_id}", self.base.base_url()));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<Vec<Infraction>>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }
}

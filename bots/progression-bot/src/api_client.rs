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
pub struct StreakResponse {
    #[serde(default)]
    pub streak_current: u32,
    #[serde(default)]
    pub streak_best: u32,
    #[serde(default)]
    pub streak_last_day: u32,
    #[serde(default)]
    pub streak_last_year: i32,
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
    #[serde(default)]
    pub xp_text: i64,
    #[serde(default)]
    pub level_text: i32,
    #[serde(default)]
    pub xp_text_current: i64,
    #[serde(default)]
    pub xp_text_needed: i64,
    #[serde(default)]
    pub xp_voice: i64,
    #[serde(default)]
    pub level_voice: i32,
    #[serde(default)]
    pub xp_voice_current: i64,
    #[serde(default)]
    pub xp_voice_needed: i64,
    #[serde(default)]
    pub streak_current: Option<i32>,
    #[serde(default)]
    pub streak_best: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AddXpResponse {
    pub user: UserLevelResponse,
    pub leveled_up: bool,
    pub old_level: i32,
    pub reward_role_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct RewardEntry {
    pub id: String,
    pub guild_id: String,
    pub level: i32,
    pub role_id: String,
    pub source: String,
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

/// Client API specifique au progression-bot. Delegue les appels generiques au BaseApiClient.
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
        self.base
            .post_fire_and_forget(
                "/api/stats/messages",
                &RecordMessagesPayload {
                    guild_id: guild_id.to_string(),
                    user_id: user_id.to_string(),
                    username: username.to_string(),
                    count,
                },
            )
            .await;
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
        self.base
            .post_fire_and_forget(
                "/api/stats/voice",
                &RecordVoicePayload {
                    guild_id: guild_id.to_string(),
                    user_id: user_id.to_string(),
                    username: username.to_string(),
                    seconds,
                    channel_id: channel_id.to_string(),
                    channel_name: channel_name.to_string(),
                },
            )
            .await;
        Ok(())
    }

    /// Recupere les stats d'un utilisateur depuis le backend.
    pub async fn get_user_stats(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserStatsResponse>, String> {
        self.base
            .get_json(&format!("/api/stats/{guild_id}/user/{user_id}"))
            .await
    }

    /// Recupere les stats globales du serveur.
    pub async fn get_guild_overview(
        &self,
        guild_id: &str,
    ) -> Result<GuildOverviewResponse, String> {
        self.base
            .get_json(&format!("/api/stats/{guild_id}/overview"))
            .await
    }

    /// Recupere le classement des membres.
    pub async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserStatsResponse>, String> {
        self.base
            .get_json(&format!("/api/stats/{guild_id}/leaderboard?limit={limit}"))
            .await
    }

    // ── Levels / XP ──

    /// Ajoute de l'XP a un utilisateur avec la source specifiee.
    pub async fn add_xp(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        amount: i64,
        source: &str,
    ) -> Result<AddXpResponse, String> {
        #[derive(Serialize)]
        struct Payload {
            guild_id: String,
            user_id: String,
            username: String,
            amount: i64,
            source: String,
        }

        self.base
            .post_json(
                "/api/levels/xp",
                &Payload {
                    guild_id: guild_id.to_string(),
                    user_id: user_id.to_string(),
                    username: username.to_string(),
                    amount,
                    source: source.to_string(),
                },
            )
            .await
    }

    pub async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserLevelResponse>, String> {
        let path = format!("/api/levels/{guild_id}/{user_id}");
        let resp = self.base.auth(
            self.base.client().get(format!("{}{}", self.base.base_url(), path))
        ).send().await.map_err(|e| format!("Erreur reseau: {e}"))?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        resp.json::<UserLevelResponse>()
            .await
            .map(Some)
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Recupere le leaderboard des niveaux, optionnellement filtre par source.
    pub async fn get_level_leaderboard(
        &self,
        guild_id: &str,
        limit: u32,
        source: Option<&str>,
    ) -> Result<Vec<UserLevelResponse>, String> {
        let mut path = format!("/api/levels/{guild_id}/leaderboard?limit={limit}");
        if let Some(s) = source {
            path.push_str(&format!("&source={s}"));
        }
        self.base.get_json(&path).await
    }

    /// Charge les donnees de streak d'un utilisateur.
    pub async fn get_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<StreakResponse, String> {
        self.base
            .get_json(&format!("/api/levels/{guild_id}/{user_id}/streak"))
            .await
    }

    /// Persiste les donnees de streak pour un utilisateur.
    pub async fn update_streak(
        &self,
        guild_id: &str,
        user_id: &str,
        current: u32,
        best: u32,
        last_day: u32,
        last_year: i32,
    ) {
        self.base
            .patch_fire_and_forget(
                &format!("/api/levels/{guild_id}/{user_id}/streak"),
                &serde_json::json!({
                    "streak_current": current,
                    "streak_best": best,
                    "streak_last_day": last_day,
                    "streak_last_year": last_year,
                }),
            )
            .await;
    }

    /// Recupere tous les rewards (text, voice, days) pour un serveur.
    pub async fn get_all_rewards(
        &self,
        guild_id: &str,
    ) -> Result<Vec<RewardEntry>, String> {
        self.base
            .get_json(&format!("/api/levels/rewards/{guild_id}"))
            .await
    }

    /// Recupere les infractions d'un serveur.
    pub async fn get_infractions(&self, guild_id: &str) -> Result<Vec<Infraction>, String> {
        self.base
            .get_json(&format!("/infractions/{guild_id}"))
            .await
    }
}

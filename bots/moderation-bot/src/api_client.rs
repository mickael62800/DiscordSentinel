use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;

/// Action de moderation envoyee au backend.
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
    /// Gravite pour les warns : "low", "medium", "high"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gravity: Option<String>,
    /// Duree en secondes (None = permanent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ModerationActionResponse {
    pub id: String,
    pub action_type: String,
    pub target_name: String,
    pub reason: String,
    pub escalation_action: Option<String>,
    pub escalation_duration: Option<u64>,
    pub strikes_count: Option<u32>,
}

/// Historique des sanctions d'un utilisateur.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UserHistory {
    pub target_id: String,
    pub target_name: String,
    pub total_warns: u32,
    pub total_mutes: u32,
    pub total_bans: u32,
    pub actions: Vec<ModerationActionResponse>,
}

/// Client API specifique a la moderation. Delegue les appels generiques au BaseApiClient.
pub struct ApiClient {
    base: BaseApiClient,
}

impl ApiClient {
    pub fn new(base: BaseApiClient) -> Self {
        Self { base }
    }

    /// Enregistre une action de moderation dans le backend.
    pub async fn log_action(&self, action: &ModerationAction) -> Result<ModerationActionResponse, String> {
        self.base.post_json("/api/moderation/actions", action).await
    }

    /// Recupere l'historique des sanctions d'un utilisateur.
    pub async fn get_history(&self, guild_id: &str, user_id: &str) -> Result<UserHistory, String> {
        self.base
            .get_json(&format!("/api/moderation/history/{}/{}", guild_id, user_id))
            .await
    }

    /// Ajoute une note sur un utilisateur.
    pub async fn add_note(
        &self,
        guild_id: &str,
        user_id: &str,
        author_id: &str,
        author_name: &str,
        content: &str,
        category: &str,
    ) -> Result<serde_json::Value, String> {
        self.base
            .post_json(
                "/api/notes",
                &serde_json::json!({
                    "guild_id": guild_id,
                    "user_id": user_id,
                    "author_id": author_id,
                    "author_name": author_name,
                    "content": content,
                    "category": category,
                }),
            )
            .await
    }

    // ── Pending Actions (mode apprenti) ──

    /// Persiste une action en attente d'approbation (fire-and-forget).
    #[allow(dead_code)]
    pub async fn create_pending_action(&self, action: &ModerationAction) {
        self.base
            .post_fire_and_forget("/api/moderation/pending", action)
            .await;
    }

    /// Met a jour le statut d'une action en attente (approved/rejected).
    pub async fn resolve_pending_action(&self, action_id: &str, status: &str, reviewed_by: &str) {
        self.base
            .patch_fire_and_forget(
                &format!("/api/moderation/pending/{action_id}"),
                &serde_json::json!({
                    "status": status,
                    "reviewed_by": reviewed_by,
                }),
            )
            .await;
    }
}

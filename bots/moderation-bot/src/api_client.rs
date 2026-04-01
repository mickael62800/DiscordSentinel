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
        let req = self
            .base
            .client()
            .post(format!("{}/api/moderation/actions", self.base.base_url()))
            .json(action);

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<ModerationActionResponse>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    /// Recupere l'historique des sanctions d'un utilisateur.
    pub async fn get_history(&self, guild_id: &str, user_id: &str) -> Result<UserHistory, String> {
        let req = self
            .base
            .client()
            .get(format!(
                "{}/api/moderation/history/{}/{}",
                self.base.base_url(),
                guild_id,
                user_id
            ));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<UserHistory>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
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
        let req = self
            .base
            .client()
            .post(format!("{}/api/notes", self.base.base_url()))
            .json(&serde_json::json!({
                "guild_id": guild_id,
                "user_id": user_id,
                "author_id": author_id,
                "author_name": author_name,
                "content": content,
                "category": category,
            }));

        self.base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur reseau: {e}"))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Erreur parsing: {e}"))
    }

    // ── Pending Actions (mode apprenti) ──

    /// Persiste une action en attente d'approbation (fire-and-forget).
    #[allow(dead_code)]
    pub async fn create_pending_action(&self, action: &ModerationAction) {
        let req = self
            .base
            .client()
            .post(format!("{}/api/moderation/pending", self.base.base_url()))
            .json(action);

        self.base.auth(req).send().await.ok();
    }

    /// Met a jour le statut d'une action en attente (approved/rejected).
    pub async fn resolve_pending_action(&self, action_id: &str, status: &str, reviewed_by: &str) {
        let req = self
            .base
            .client()
            .patch(format!("{}/api/moderation/pending/{action_id}", self.base.base_url()))
            .json(&serde_json::json!({
                "status": status,
                "reviewed_by": reviewed_by,
            }));

        self.base.auth(req).send().await.ok();
    }
}

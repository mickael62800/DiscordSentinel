use std::sync::Arc;

use serde::{Deserialize, Serialize};

use sentinel_shared::api_client::BaseApiClient;
use sentinel_shared::grpc_client::{GrpcCallError, SentinelGrpcClient};

use sentinel_proto::moderation::v1 as proto_mod;

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
    pub moderator_name: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub created_at: String,
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

/// MOD #2 — Preuve attachee a une action de moderation.
#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct EvidenceEntry {
    pub id: String,
    pub action_id: String,
    pub url: String,
    pub description: Option<String>,
    pub uploaded_by: String,
    pub uploaded_by_name: String,
    pub uploaded_at: String,
}

/// MOD #3 — Entree de la file de relecture.
#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ReviewQueueEntry {
    pub id: String,
    pub action_id: String,
    pub guild_id: String,
    pub added_by: String,
    pub added_by_name: String,
    pub reason: Option<String>,
    pub status: String,
    pub reviewer_id: Option<String>,
    pub reviewer_name: Option<String>,
    pub reviewer_notes: Option<String>,
    pub added_at: String,
    pub resolved_at: Option<String>,
    pub action_type: Option<String>,
    pub target_name: Option<String>,
    pub action_reason: Option<String>,
}

/// MOD #7 — Agregation d'actions de moderation par moderateur.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ModStatsEntry {
    pub moderator_id: String,
    pub moderator_name: String,
    pub total: i64,
    pub warns: i64,
    pub mutes: i64,
    pub bans: i64,
    pub kicks: i64,
}

/// Sanction temporaire active (reminder pending).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SanctionReminder {
    pub id: String,
    pub guild_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub action_id: String,
    pub remind_at: String,
    pub expires_at: String,
    pub status: String,
    pub created_at: String,
}

/// Client API de la moderation.
///
/// Phase 7A — Migration gRPC partielle :
/// - `log_action` (hot path : chaque ban/mute/warn) -> `ModerationService.LogAction`
/// - `get_history` (consultation frequente) -> `ModerationService.GetHistory`
/// - tout le reste reste HTTP via `BaseApiClient` : evidence, review queue,
///   modstats, reminders, notes, bot config, pending actions. Ces endpoints
///   utilisent des repos directs cote API et ne sont pas exposes par le
///   `ManageModerationUseCase` v1 — ils seront migres dans une iteration
///   ulterieure quand le domaine sera consolide.
///
/// ## Comportement si l'API tombe
///
/// - `log_action` (gRPC) : circuit breaker apres 5 echecs, ouvert 10s. Une
///   sanction ratee est loggee en erreur — elle reste appliquee cote Discord
///   (le ban/mute a deja eu lieu via Serenity), seul le log backend est
///   manquant. Le moderateur est notifie via le retour Err.
/// - `get_history` (gRPC) : retourne `Err("API indisponible...")`, la
///   commande slash repond a l'utilisateur clairement.
/// - Endpoints HTTP (evidence, review, etc.) : comportement inchange,
///   `BaseApiClient` retry une fois puis remonte l'erreur.
pub struct ApiClient {
    base: BaseApiClient,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(base: BaseApiClient, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    /// Enregistre une action de moderation dans le backend (gRPC).
    pub async fn log_action(
        &self,
        action: &ModerationAction,
    ) -> Result<ModerationActionResponse, String> {
        let req = proto_mod::LogActionRequest {
            guild_id: action.guild_id.clone(),
            channel_id: action.channel_id.clone(),
            moderator_id: action.moderator_id.clone(),
            moderator_name: action.moderator_name.clone(),
            target_id: action.target_id.clone(),
            target_name: action.target_name.clone(),
            action_type: action.action_type.clone(),
            reason: action.reason.clone(),
            gravity: action.gravity.clone(),
            duration: action.duration,
        };
        let mut client = self.grpc.moderation();
        let resp = self
            .grpc
            .guarded(|| async move { client.log_action(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(ModerationActionResponse {
            id: resp.id,
            action_type: resp.action_type,
            target_name: resp.target_name,
            moderator_name: resp.moderator_name,
            reason: resp.reason,
            gravity: resp.gravity,
            created_at: resp.created_at,
            // Phase 7B : strikes_count + escalation remontes par le serveur gRPC.
            escalation_action: resp.escalation_action,
            escalation_duration: resp.escalation_duration,
            strikes_count: resp.strikes_count,
        })
    }

    /// Recupere l'historique des sanctions d'un utilisateur (gRPC).
    pub async fn get_history(&self, guild_id: &str, user_id: &str) -> Result<UserHistory, String> {
        let req = proto_mod::GetHistoryRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
        };
        let mut client = self.grpc.moderation();
        let history = self
            .grpc
            .guarded(|| async move { client.get_history(req).await.map(|r| r.into_inner()) })
            .await
            .map_err(grpc_err_to_string)?;
        Ok(UserHistory {
            target_id: history.target_id,
            target_name: history.target_name,
            total_warns: history.total_warns,
            total_mutes: history.total_mutes,
            total_bans: history.total_bans,
            actions: history
                .actions
                .into_iter()
                .map(|a| ModerationActionResponse {
                    id: a.id,
                    action_type: a.action_type,
                    target_name: a.target_name,
                    moderator_name: a.moderator_name,
                    reason: a.reason,
                    gravity: a.gravity,
                    created_at: a.created_at,
                    escalation_action: None,
                    escalation_duration: None,
                    strikes_count: None,
                })
                .collect(),
        })
    }

    /// Supprime une action de moderation par son ID (unwarn).
    pub async fn delete_action(&self, action_id: &str) -> Result<bool, String> {
        let req = self.base.client()
            .delete(format!("{}/api/moderation/actions/{}", self.base.base_url(), action_id));
        let resp = self.base.auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP delete_action: {e}"))?;
        Ok(resp.status().is_success())
    }

    /// Reset tous les strikes actifs d'un utilisateur (purge user_strikes).
    /// Utilise par /unwarn all pour garantir que meme les strikes orphelins
    /// (sans infraction_id, crees avant le fix de liaison) sont retires.
    pub async fn reset_strikes(&self, guild_id: &str, user_id: &str) -> Result<(), String> {
        let req = self.base.client().delete(format!(
            "{}/api/strikes/{}/{}",
            self.base.base_url(),
            guild_id,
            user_id
        ));
        let resp = self
            .base
            .auth(req)
            .send()
            .await
            .map_err(|e| format!("Erreur HTTP reset_strikes: {e}"))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("reset_strikes: HTTP {}", resp.status()))
        }
    }

    /// MOD #1 — Liste les sanctions temporaires actives (reminders pending) d'une guild.
    pub async fn get_active_reminders(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, String> {
        self.base
            .get_json(&format!("/api/reminders/{}", guild_id))
            .await
    }

    /// MOD #7 — Top 20 des moderateurs par nombre d'actions sur les 30 derniers jours.
    pub async fn get_modstats(&self, guild_id: &str) -> Result<Vec<ModStatsEntry>, String> {
        self.base
            .get_json(&format!("/api/moderation/modstats/{}", guild_id))
            .await
    }

    /// MOD #2 — Attache une preuve a une action de moderation existante.
    pub async fn add_evidence(
        &self,
        action_id: &str,
        url: &str,
        description: Option<&str>,
        uploaded_by: &str,
        uploaded_by_name: &str,
    ) -> Result<EvidenceEntry, String> {
        self.base
            .post_json(
                "/api/moderation/evidence",
                &serde_json::json!({
                    "action_id": action_id,
                    "url": url,
                    "description": description,
                    "uploaded_by": uploaded_by,
                    "uploaded_by_name": uploaded_by_name,
                }),
            )
            .await
    }

    /// MOD #2 — Liste les preuves attachees a une action.
    pub async fn list_evidence(&self, action_id: &str) -> Result<Vec<EvidenceEntry>, String> {
        self.base
            .get_json(&format!("/api/moderation/evidence/{}", action_id))
            .await
    }

    /// MOD #3 — Ajoute une action a la file de relecture.
    pub async fn add_review(
        &self,
        action_id: &str,
        guild_id: &str,
        added_by: &str,
        added_by_name: &str,
        reason: Option<&str>,
    ) -> Result<ReviewQueueEntry, String> {
        self.base
            .post_json(
                "/api/moderation/review",
                &serde_json::json!({
                    "action_id": action_id,
                    "guild_id": guild_id,
                    "added_by": added_by,
                    "added_by_name": added_by_name,
                    "reason": reason,
                }),
            )
            .await
    }

    /// MOD #3 — Liste les reviews en attente d'une guild.
    pub async fn list_pending_reviews(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ReviewQueueEntry>, String> {
        self.base
            .get_json(&format!("/api/moderation/review/{}/pending", guild_id))
            .await
    }

    /// MOD #6 — Ecrit une cle de config bot (moderateurs seniors uniquement
    /// via default_member_permissions Administrator cote commande Discord).
    /// Fire-and-forget cote bot (l'API confirme avec 204 No Content).
    pub async fn set_bot_config(
        &self,
        guild_id: &str,
        bot_name: &str,
        config_key: &str,
        config_value: &str,
    ) {
        self.base
            .post_fire_and_forget(
                "/api/bots/config",
                &serde_json::json!({
                    "guild_id": guild_id,
                    "bot_name": bot_name,
                    "config_key": config_key,
                    "config_value": config_value,
                }),
            )
            .await;
    }

    /// MOD #3 — Resout une review en fire-and-forget.
    pub async fn resolve_review(
        &self,
        review_id: &str,
        status: &str,
        reviewer_id: &str,
        reviewer_name: &str,
        notes: Option<&str>,
    ) {
        self.base
            .patch_fire_and_forget(
                &format!("/api/moderation/review/{}/resolve", review_id),
                &serde_json::json!({
                    "status": status,
                    "reviewer_id": reviewer_id,
                    "reviewer_name": reviewer_name,
                    "reviewer_notes": notes,
                }),
            )
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

fn grpc_err_to_string(e: GrpcCallError) -> String {
    match e {
        GrpcCallError::Unavailable => "API indisponible (circuit breaker ouvert)".to_string(),
        GrpcCallError::Status(s) => format!("gRPC {:?}: {}", s.code(), s.message()),
        GrpcCallError::Transport(t) => format!("transport gRPC: {t}"),
    }
}

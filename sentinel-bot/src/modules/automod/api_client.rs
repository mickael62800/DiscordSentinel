//! Client API du automod module.
//!
//! Phase 7A -- Migration gRPC complete : `analyze` est le **hot path le plus
//! chaud du projet** (un appel par message Discord recu sur tous les
//! serveurs). Le gain perf gRPC est ici maximal.
//!
//! ## Comportement si l'API tombe
//!
//! Le circuit breaker (5 echecs / 10s) court-circuite immediatement les
//! appels suivants. Pendant l'ouverture, `analyze` retourne `Err("API
//! indisponible")` et le bot **n'applique aucune action de moderation**.
//! Comportement par defaut : laisser passer le message (ne pas faire de
//! faux positifs basees sur une API down). Cote handler, le timeout
//! original de 5s est conserve pour ne pas bloquer le bot.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::shared::api_client::BaseApiClient;
use crate::shared::grpc_client::SentinelGrpcClient;

use sentinel_proto::automod::v1 as proto;

use super::detectors::DetectionFlags;

#[derive(Debug, Serialize)]
pub struct AnalyzeRequest {
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub flags: DetectionFlags,
    pub metadata: MessageMetadata,
    pub context_messages: Vec<ContextMessage>,
}

#[derive(Debug, Serialize)]
pub struct ContextMessage {
    pub username: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct MessageMetadata {
    pub message_id: String,
    pub timestamp: String,
}

/// Decision de routage calculee cote serveur (decide = API). Le bot execute.
#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
pub enum Routing {
    /// Ne rien faire automatiquement.
    None,
    /// Poster une carte de review/vote.
    Card,
    /// Appliquer directement l'action (mode auto).
    Auto,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AnalyzeResponse {
    pub action: Action,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub score: Option<f64>,
    /// Decision de routage (cote serveur).
    pub route: Routing,
    /// Cas severe -> protection auto (mute + suppression) immediate.
    pub severe: bool,
    /// Lien non autorise hors image -> suppression auto immediate.
    pub auto_delete_link: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    None,
    Warn,
    Delete,
    Mute,
    Ban,
}

pub struct ApiClient {
    #[allow(dead_code)]
    pub base: Arc<BaseApiClient>,
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(base: Arc<BaseApiClient>, grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { base, grpc }
    }

    /// gRPC `AutomodService.AnalyzeMessage` (hot path le plus chaud).
    pub async fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzeResponse, String> {
        let req = proto::AnalyzeMessageRequest {
            guild_id: request.guild_id.clone(),
            channel_id: request.channel_id.clone(),
            user_id: request.user_id.clone(),
            username: request.username.clone(),
            content: request.content.clone(),
            flags: Some(proto::DetectionFlags {
                spam: request.flags.spam,
                insult: request.flags.insult,
                profanity: request.flags.profanity,
                link: request.flags.link,
                phishing: request.flags.phishing,
            }),
            message_id: request.metadata.message_id.clone(),
            timestamp: request.metadata.timestamp.clone(),
            context_messages: request
                .context_messages
                .iter()
                .map(|m| proto::ContextMessage {
                    username: m.username.clone(),
                    content: m.content.clone(),
                })
                .collect(),
        };
        let resp = crate::grpc_call!(self.grpc, automod, analyze_message, req)?;
        Ok(AnalyzeResponse {
            action: proto_action_to_action(resp.action),
            reason: if resp.reason.is_empty() {
                None
            } else {
                Some(resp.reason)
            },
            duration: resp.duration,
            score: Some(resp.score),
            route: proto_routing_to_routing(resp.route),
            severe: resp.severe,
            auto_delete_link: resp.auto_delete_link,
        })
    }

    /// gRPC `AutomodService.EvaluateFlood` : verdict d'auto-protection face a
    /// un flood. Retourne `(severe, mute_duration_secs)`. La regle (seuil
    /// severe + toggle) vit cote serveur.
    pub async fn evaluate_flood(
        &self,
        guild_id: &str,
        user_id: &str,
        channel_id: &str,
        flood_count: i32,
    ) -> Result<(bool, i64, f64), String> {
        let req = proto::EvaluateFloodRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            channel_id: channel_id.to_string(),
            flood_count,
        };
        let resp = crate::grpc_call!(self.grpc, automod, evaluate_flood, req)?;
        Ok((resp.severe, resp.mute_duration_secs, resp.score))
    }

    /// gRPC `AutomodService.EvaluateAttachments` : verdict sur des pieces
    /// jointes suspectes. La regle (extensions dangereuses + config) vit cote
    /// serveur ; le bot n'EXECUTE que l'action renvoyee.
    pub async fn evaluate_attachments(
        &self,
        guild_id: &str,
        filenames: Vec<String>,
    ) -> Result<AttachmentVerdict, String> {
        let req = proto::EvaluateAttachmentsRequest {
            guild_id: guild_id.to_string(),
            filenames,
        };
        let resp = crate::grpc_call!(self.grpc, automod, evaluate_attachments, req)?;
        Ok(AttachmentVerdict {
            suspicious: resp.suspicious,
            action: proto_action_to_action(resp.action),
            reason: resp.reason,
            score: resp.score,
            filename: resp.filename,
        })
    }

    /// gRPC `AutomodService.EvaluateCaps` : score de confiance a afficher pour
    /// une detection de CAPS. La detection reste locale (rate/forme) ; le SCORE
    /// affiche est fabrique cote serveur (avant : 0.8 code en dur dans le bot).
    pub async fn evaluate_caps(&self, guild_id: &str) -> Result<f64, String> {
        let req = proto::EvaluateCapsRequest {
            guild_id: guild_id.to_string(),
        };
        let resp = crate::grpc_call!(self.grpc, automod, evaluate_caps, req)?;
        Ok(resp.score)
    }

    // analyze_image supprime -- migre vers ai-worker (async queue + Redis).
}

/// Verdict d'analyse de pieces jointes renvoye par l'API.
#[derive(Debug)]
pub struct AttachmentVerdict {
    pub suspicious: bool,
    pub action: Action,
    pub reason: String,
    pub score: f64,
    pub filename: String,
}

fn proto_action_to_action(value: i32) -> Action {
    match proto::Action::try_from(value).unwrap_or(proto::Action::None) {
        proto::Action::None => Action::None,
        proto::Action::Warn => Action::Warn,
        proto::Action::Delete => Action::Delete,
        proto::Action::Mute => Action::Mute,
        proto::Action::Ban => Action::Ban,
    }
}

fn proto_routing_to_routing(value: i32) -> Routing {
    match proto::Routing::try_from(value).unwrap_or(proto::Routing::None) {
        proto::Routing::None => Routing::None,
        proto::Routing::Card => Routing::Card,
        proto::Routing::Auto => Routing::Auto,
    }
}

use crate::shared::grpc_client::grpc_err_to_string;

// ── Persistance du slowmode adaptatif (BUG3) ──
// Le tracker est en memoire ; on mirroir l'ensemble actif cote API pour le
// recharger apres un redemarrage (sinon salons bloques en slowmode a vie).

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AdaptiveSlowmodeEntry {
    #[serde(default)]
    pub guild_id: String,
    pub channel_id: String,
}

/// Marque un salon comme slowmode adaptatif actif (best-effort).
pub async fn persist_slowmode(api: &BaseApiClient, guild_id: &str, channel_id: &str) {
    let body = AdaptiveSlowmodeEntry {
        guild_id: guild_id.to_string(),
        channel_id: channel_id.to_string(),
    };
    let _ = api
        .post_json::<_, serde_json::Value>("/api/automod/adaptive-slowmode", &body)
        .await;
}

/// Retire un salon (slowmode desactive) — best-effort.
pub async fn forget_slowmode(api: &BaseApiClient, channel_id: &str) {
    let body = AdaptiveSlowmodeEntry {
        guild_id: String::new(),
        channel_id: channel_id.to_string(),
    };
    let _ = api
        .post_json::<_, serde_json::Value>("/api/automod/adaptive-slowmode/remove", &body)
        .await;
}

/// Liste tous les salons actifs (rechargement au demarrage).
pub async fn list_slowmode(api: &BaseApiClient) -> Vec<AdaptiveSlowmodeEntry> {
    api.get_json("/api/automod/adaptive-slowmode")
        .await
        .unwrap_or_default()
}

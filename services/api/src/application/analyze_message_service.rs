use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, info};
use uuid::Uuid;

use crate::domain::entities::{Infraction, MessageAnalysis};
use crate::domain::errors::DomainError;
use crate::adapters::outbound::{InferenceService, TextTokenizer};
use crate::domain::services::{InferenceRateLimiter, ScoringService};
use crate::domain::value_objects::{Action, FlagType};
use crate::ports::inbound::{AnalyzeMessageCommand, AnalyzeMessageUseCase, DeductPointsCommand, ManageConductUseCase};
use crate::ports::outbound::{CachePort, IaConfigRepository, InfractionRepository, RuleRepository};

/// Seuil de confiance par defaut (utilise si pas de config per-guild).
const DEFAULT_TEXT_THRESHOLD: f32 = 0.5;

pub struct AnalyzeMessageService {
    rule_repo: Arc<dyn RuleRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
    conduct_uc: Arc<dyn ManageConductUseCase>,
    ia_config_repo: Arc<dyn IaConfigRepository>,
    inference_limiter: Arc<InferenceRateLimiter>,
    inference: Option<Arc<InferenceService>>,
    tokenizer: Option<Arc<TextTokenizer>>,
}

impl AnalyzeMessageService {
    pub fn new(
        rule_repo: Arc<dyn RuleRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
        conduct_uc: Arc<dyn ManageConductUseCase>,
        ia_config_repo: Arc<dyn IaConfigRepository>,
        inference_limiter: Arc<InferenceRateLimiter>,
    ) -> Self {
        Self {
            rule_repo,
            infraction_repo,
            cache,
            conduct_uc,
            ia_config_repo,
            inference_limiter,
            inference: None,
            tokenizer: None,
        }
    }

    /// Ajoute l'inference text IA au service d'analyse.
    pub fn with_text_inference(
        mut self,
        inference: Arc<InferenceService>,
        tokenizer: Arc<TextTokenizer>,
    ) -> Self {
        self.inference = Some(inference);
        self.tokenizer = Some(tokenizer);
        self
    }
}

#[async_trait]
impl AnalyzeMessageUseCase for AnalyzeMessageService {
    async fn analyze(&self, cmd: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
        // 1. Charger les règles (cache → DB)
        let rules = match self.cache.get_rules(&cmd.guild_id).await? {
            Some(cached) => cached,
            None => {
                let from_db = self.rule_repo.find_by_guild(&cmd.guild_id).await?;
                if let Err(e) = self.cache.set_rules(&cmd.guild_id, &from_db).await {
                    tracing::warn!(error = %e, guild_id = %cmd.guild_id, "Echec cache set rules");
                }
                from_db
            }
        };

        // 2. Scoring basique (flags bot : spam, insult, link, phishing)
        let mut result = ScoringService::score(&cmd.flags, &rules);

        // 3. Inference text IA (sentiment : anger, rage, threat, harassment)
        // Charger la config IA per-guild pour le seuil de confiance
        let ia_config = match self.ia_config_repo.get(&cmd.guild_id).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %cmd.guild_id, "Echec chargement config IA, utilisation defauts");
                None
            }
        };
        let text_enabled = ia_config.as_ref().map(|c| c.text_enabled).unwrap_or(true);
        let text_threshold = ia_config.as_ref().map(|c| c.text_threshold as f32).unwrap_or(DEFAULT_TEXT_THRESHOLD);
        let context_dampening = ia_config.as_ref().map(|c| c.context_dampening).unwrap_or(0.65);
        let context_format = ia_config.as_ref().map(|c| c.context_format.clone()).unwrap_or_else(|| "natural".to_string());
        // Duree de mute configurable (defaut 600s = 10 min).
        // Pas dans ia_config (schema fixe), lu depuis scoring ou defaut.
        let mute_duration_secs: u64 = 600;

        debug!(
            has_inference = self.inference.is_some(),
            has_tokenizer = self.tokenizer.is_some(),
            text_enabled,
            "Etat inference IA"
        );

        if let (Some(inference), Some(tokenizer)) = (&self.inference, &self.tokenizer) {
            debug!(
                text_available = inference.text_available(),
                tokenizer_available = tokenizer.available(),
                content_empty = cmd.content.is_empty(),
                "Check inference conditions"
            );
            if text_enabled && inference.text_available() && tokenizer.available() && !cmd.content.is_empty() {
                // Rate limit inference
                let _permit = self.inference_limiter.acquire().await?;

                debug!("Lancement inference text...");
                let contextual_content = build_contextual_content(&cmd.content, &cmd.context_messages, &context_format);
                let has_context = !cmd.context_messages.is_empty();
                // Timeout 5s pour eviter qu'une inference bloquee ne stalle le hot path.
                let inference_result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    tokio::task::spawn_blocking({
                        let inf = Arc::clone(inference);
                        let tok = Arc::clone(tokenizer);
                        let rules = rules.clone();
                        let content = contextual_content.clone();
                        move || {
                            let (input_ids, attention_mask) = tok.tokenize(&content)?;
                            let classifications = inf.classify_text(input_ids, attention_mask)?;
                            Ok::<_, String>(score_classifications(&classifications, &rules, text_threshold))
                        }
                    }),
                )
                .await;
                let inference_result = match inference_result {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => Err(format!("spawn_blocking: {e}")),
                    Err(_) => Err("Inference text timeout (5s)".to_string()),
                };
                match inference_result {
                    Ok(Some((ia_score, _ia_flags, ia_reason))) => {
                        // Attenuer le score IA si du contexte conversationnel est disponible
                        // (reduit les faux positifs sur les blagues entre amis, etc.)
                        let ia_score = if has_context && context_dampening < 1.0 {
                            let dampened = ia_score * context_dampening;
                            debug!(
                                original_ia_score = ia_score,
                                dampened_ia_score = dampened,
                                context_dampening,
                                "Score IA attenue grace au contexte conversationnel"
                            );
                            dampened
                        } else {
                            ia_score
                        };

                        // Combiner : prendre le score le plus eleve
                        let combined_score = result.score + ia_score;

                        info!(
                            bot_score = result.score,
                            ia_score = ia_score,
                            combined = combined_score,
                            ia_flags = %ia_reason,
                            "Scoring combine bot + IA text"
                        );

                        // Recalculer l'action avec le score combine
                        let (t_warn, t_delete, t_mute, t_ban) = resolve_thresholds(&rules);

                        let (action, duration) = if combined_score >= t_ban {
                            (Action::Ban, None)
                        } else if combined_score >= t_mute {
                            (Action::Mute, Some(mute_duration_secs))
                        } else if combined_score >= t_delete {
                            (Action::Delete, None)
                        } else if combined_score >= t_warn {
                            (Action::Warn, None)
                        } else {
                            (Action::None, None)
                        };

                        // Combiner les raisons
                        let reason = if result.reason.is_empty() {
                            ia_reason
                        } else {
                            format!("{} + {}", result.reason, ia_reason)
                        };

                        result.score = combined_score;
                        result.action = action;
                        result.reason = reason;
                        result.duration = duration;
                    }
                    Ok(None) => {
                        // Pas de sentiment toxique detecte
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Inference text echouee — scoring bot seul");
                    }
                }
            }
        }

        // 4. Persister l'infraction
        let infraction = Infraction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            channel_id: cmd.channel_id,
            user_id: cmd.user_id,
            username: cmd.username,
            message_id: cmd.message_id,
            content: cmd.content,
            flags: cmd.flags,
            score: result.score,
            action: result.action.clone(),
            reason: result.reason.clone(),
            duration: result.duration,
            created_at: chrono::Utc::now(),
        };

        self.infraction_repo.save(&infraction).await?;

        // 4b. Deduire les points de conduite
        if result.action.as_str() != "none" {
            if let Err(e) = self.conduct_uc.deduct_points(DeductPointsCommand {
                guild_id: infraction.guild_id.clone(),
                user_id: infraction.user_id.clone(),
                username: infraction.username.clone(),
                action: result.action.as_str().to_string(),
            }).await {
                tracing::warn!(error = %e, guild_id = %infraction.guild_id, user_id = %infraction.user_id, "Echec deduction points conduite (analyse message)");
            }
        }

        // 5. Retourner l'analyse
        Ok(MessageAnalysis {
            action: result.action,
            reason: result.reason,
            score: result.score,
            duration: result.duration,
        })
    }
}

// run_text_inference supprimee — remplacee par spawn_blocking + timeout dans analyze().

/// Fonction pure : transforme les classifications IA en score, flags et raison.
/// Retourne None si aucun sentiment toxique n'est detecte au-dessus du seuil.
pub fn score_classifications(
    classifications: &[crate::adapters::outbound::InferenceClassification],
    rules: &[crate::domain::entities::Rule],
    threshold: f32,
) -> Option<(f64, Vec<FlagType>, String)> {
    let mut detected: Vec<(FlagType, f32)> = Vec::new();

    for c in classifications {
        let flag = match c.label.as_str() {
            "anger" if c.confidence >= threshold => Some(FlagType::Anger),
            "rage" if c.confidence >= threshold => Some(FlagType::Rage),
            "threat" if c.confidence >= threshold => Some(FlagType::Threat),
            "harassment" if c.confidence >= threshold => Some(FlagType::Harassment),
            _ => None,
        };

        if let Some(flag_type) = flag {
            detected.push((flag_type, c.confidence));
        }
    }

    if detected.is_empty() {
        return None;
    }

    let mut ia_score = 0.0;
    let mut triggered: Vec<String> = Vec::new();

    for (flag_type, confidence) in &detected {
        let rule = rules.iter().find(|r| r.flag_type == *flag_type && r.enabled);
        let base_weight = match rule {
            Some(r) => r.weight,
            None => match flag_type {
                FlagType::Anger => 3.0,
                FlagType::Rage => 6.0,
                FlagType::Threat => 8.0,
                FlagType::Harassment => 7.0,
                _ => 5.0,
            },
        };
        let weighted = base_weight * (*confidence as f64);
        ia_score += weighted;
        triggered.push(format!("{}({:.0}%)", flag_type.as_str(), confidence * 100.0));
    }

    let reason = format!("IA sentiment : {}", triggered.join(", "));
    Some((ia_score, detected.into_iter().map(|(f, _)| f).collect(), reason))
}

/// Construit un contenu enrichi avec le contexte conversationnel pour l'inference IA.
/// Le message analyse est place en premier (safe si le tokenizer tronque la fin).
/// - "natural" : conversation brute separee par des retours a la ligne
/// - "tagged"  : balises [message]/[context] pour structurer l'input
fn build_contextual_content(
    content: &str,
    context: &[crate::ports::inbound::ContextMessageEntry],
    format: &str,
) -> String {
    if context.is_empty() {
        return content.to_string();
    }
    let ctx_str: String = context
        .iter()
        .map(|m| format!("{}: {}", m.username, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    match format {
        "tagged" => format!("[message] {} [/message] [context] {} [/context]", content, ctx_str),
        _ => format!("{}\n---\n{}", ctx_str, content),
    }
}

/// Seuils par defaut (replique du ScoringService pour le scoring combine).
const DEFAULT_THRESHOLD_WARN: f64 = 2.0;
const DEFAULT_THRESHOLD_DELETE: f64 = 4.0;
const DEFAULT_THRESHOLD_MUTE: f64 = 6.0;
const DEFAULT_THRESHOLD_BAN: f64 = 9.0;

fn resolve_thresholds(rules: &[crate::domain::entities::Rule]) -> (f64, f64, f64, f64) {
    let enabled: Vec<_> = rules.iter().filter(|r| r.enabled).collect();

    if enabled.is_empty() {
        return (
            DEFAULT_THRESHOLD_WARN,
            DEFAULT_THRESHOLD_DELETE,
            DEFAULT_THRESHOLD_MUTE,
            DEFAULT_THRESHOLD_BAN,
        );
    }

    let warn = enabled.iter().map(|r| r.threshold_warn).fold(f64::MAX, f64::min);
    let delete = enabled.iter().map(|r| r.threshold_delete).fold(f64::MAX, f64::min);
    let mute = enabled.iter().map(|r| r.threshold_mute).fold(f64::MAX, f64::min);
    let ban = enabled.iter().map(|r| r.threshold_ban).fold(f64::MAX, f64::min);

    (warn, delete, mute, ban)
}


#[cfg(test)]
#[path = "tests/analyze_message_service.rs"]
mod tests;

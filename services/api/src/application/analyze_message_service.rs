use std::sync::Arc;

use async_trait::async_trait;
use tracing::{debug, info};
use uuid::Uuid;

use crate::domain::entities::{Infraction, MessageAnalysis};
use crate::domain::errors::DomainError;
use crate::domain::services::{InferenceRateLimiter, InferenceService, ScoringService, TextTokenizer};
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
    classifications: &[crate::domain::services::InferenceClassification],
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

// ── Mocks minimaux pour les tests ──

#[cfg(test)]
struct MockIaConfigRepo;

#[cfg(test)]
#[async_trait]
impl IaConfigRepository for MockIaConfigRepo {
    async fn get(&self, _: &str) -> Result<Option<crate::domain::entities::IaConfig>, crate::domain::errors::DomainError> { Ok(None) }
    async fn save(&self, config: &crate::domain::entities::IaConfig) -> Result<crate::domain::entities::IaConfig, crate::domain::errors::DomainError> { Ok(config.clone()) }
}

#[cfg(test)]
struct MockRuleRepo;

#[cfg(test)]
#[async_trait]
impl RuleRepository for MockRuleRepo {
    async fn find_by_guild(&self, _: &str) -> Result<Vec<crate::domain::entities::Rule>, crate::domain::errors::DomainError> { Ok(vec![]) }
    async fn find_all(&self) -> Result<Vec<crate::domain::entities::Rule>, crate::domain::errors::DomainError> { Ok(vec![]) }
    async fn find_by_id(&self, _: uuid::Uuid) -> Result<Option<crate::domain::entities::Rule>, crate::domain::errors::DomainError> { Ok(None) }
    async fn save(&self, rule: &crate::domain::entities::Rule) -> Result<crate::domain::entities::Rule, crate::domain::errors::DomainError> { Ok(rule.clone()) }
    async fn toggle(&self, _: uuid::Uuid, _: bool) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
    async fn delete(&self, _: uuid::Uuid) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
}

#[cfg(test)]
struct MockInfractionRepo;

#[cfg(test)]
#[async_trait]
impl InfractionRepository for MockInfractionRepo {
    async fn save(&self, _: &crate::domain::entities::Infraction) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
    async fn find_by_guild(&self, _: &str, _: &crate::ports::inbound::InfractionFilters) -> Result<Vec<crate::domain::entities::Infraction>, crate::domain::errors::DomainError> { Ok(vec![]) }
    async fn find_all(&self, _: i64, _: i64) -> Result<Vec<crate::domain::entities::Infraction>, crate::domain::errors::DomainError> { Ok(vec![]) }
    async fn count_today(&self) -> Result<u64, crate::domain::errors::DomainError> { Ok(0) }
    async fn find_by_id(&self, _: &str) -> Result<Option<crate::domain::entities::Infraction>, crate::domain::errors::DomainError> { Ok(None) }
    async fn delete_by_id(&self, _: &str) -> Result<bool, crate::domain::errors::DomainError> { Ok(false) }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, crate::domain::errors::DomainError> { Ok(0) }
}

#[cfg(test)]
struct MockCache;

#[cfg(test)]
#[async_trait]
impl CachePort for MockCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<crate::domain::entities::Rule>>, crate::domain::errors::DomainError> { Ok(None) }
    async fn set_rules(&self, _: &str, _: &[crate::domain::entities::Rule]) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
    async fn invalidate_rules(&self, _: &str) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
    async fn get_json(&self, _: &str) -> Result<Option<String>, crate::domain::errors::DomainError> { Ok(None) }
    async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
    async fn invalidate(&self, _: &str) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
}

#[cfg(test)]
struct MockConduct;

#[cfg(test)]
#[async_trait]
impl ManageConductUseCase for MockConduct {
    async fn get_config(&self, _: &str) -> Result<crate::domain::entities::ConductConfig, crate::domain::errors::DomainError> { unimplemented!() }
    async fn save_config(&self, _: crate::ports::inbound::SaveConductConfigCommand) -> Result<crate::domain::entities::ConductConfig, crate::domain::errors::DomainError> { unimplemented!() }
    async fn get_points(&self, _: &str, _: &str) -> Result<crate::domain::entities::UserConductPoints, crate::domain::errors::DomainError> { unimplemented!() }
    async fn get_leaderboard(&self, _: &str, _: i64) -> Result<Vec<crate::domain::entities::UserConductPoints>, crate::domain::errors::DomainError> { unimplemented!() }
    async fn get_points_log(&self, _: &str, _: &str, _: i64) -> Result<Vec<crate::domain::entities::ConductPointsLog>, crate::domain::errors::DomainError> { unimplemented!() }
    async fn deduct_points(&self, _: DeductPointsCommand) -> Result<crate::domain::entities::UserConductPoints, crate::domain::errors::DomainError> {
        let now = chrono::Utc::now();
        Ok(crate::domain::entities::UserConductPoints { id: uuid::Uuid::new_v4(), guild_id: String::new(), user_id: String::new(), username: String::new(), points: 100, last_regen_at: now, created_at: now, updated_at: now })
    }
    async fn add_points(&self, _: crate::ports::inbound::AddPointsCommand) -> Result<crate::domain::entities::UserConductPoints, crate::domain::errors::DomainError> { unimplemented!() }
    async fn run_regen(&self) -> Result<u64, crate::domain::errors::DomainError> { Ok(0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use crate::domain::entities::Rule;
    use crate::domain::services::InferenceClassification;

    fn make_rule(flag_type: FlagType, weight: f64) -> Rule {
        let now = Utc::now();
        Rule {
            id: Uuid::new_v4(),
            guild_id: "test".to_string(),
            flag_type,
            weight,
            threshold_warn: 2.0,
            threshold_delete: 4.0,
            threshold_mute: 6.0,
            threshold_ban: 9.0,
            enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    fn cls(label: &str, confidence: f32) -> InferenceClassification {
        InferenceClassification { label: label.to_string(), confidence }
    }

    // ── resolve_thresholds ──

    #[test]
    fn test_resolve_thresholds_defaults() {
        let (w, d, m, b) = resolve_thresholds(&[]);
        assert_eq!(w, 2.0);
        assert_eq!(d, 4.0);
        assert_eq!(m, 6.0);
        assert_eq!(b, 9.0);
    }

    #[test]
    fn test_resolve_thresholds_with_rules() {
        let rules = vec![make_rule(FlagType::Spam, 3.0)];
        let (w, d, m, b) = resolve_thresholds(&rules);
        assert_eq!(w, 2.0);
        assert_eq!(d, 4.0);
        assert_eq!(m, 6.0);
        assert_eq!(b, 9.0);
    }

    #[test]
    fn test_resolve_thresholds_ignores_disabled() {
        let mut rule = make_rule(FlagType::Spam, 3.0);
        rule.threshold_warn = 0.5;
        rule.enabled = false;
        let (w, _, _, _) = resolve_thresholds(&[rule]);
        assert_eq!(w, 2.0);
    }

    #[test]
    fn test_resolve_thresholds_takes_minimum() {
        let mut r1 = make_rule(FlagType::Spam, 3.0);
        r1.threshold_warn = 1.5;
        r1.threshold_ban = 7.0;

        let mut r2 = make_rule(FlagType::Insult, 5.0);
        r2.threshold_warn = 3.0;
        r2.threshold_ban = 10.0;

        let (w, _, _, b) = resolve_thresholds(&[r1, r2]);
        assert_eq!(w, 1.5);
        assert_eq!(b, 7.0);
    }

    #[test]
    fn test_default_text_threshold() {
        assert_eq!(DEFAULT_TEXT_THRESHOLD, 0.5);
    }

    #[test]
    fn test_with_text_inference_sets_fields() {
        use std::sync::Arc;
        use crate::domain::services::{InferenceRateLimiter, InferenceService, TextTokenizer};

        let inference = Arc::new(InferenceService::new(None, None));
        let tokenizer = Arc::new(TextTokenizer::new(None, 256));

        let _service = AnalyzeMessageService::new(
            Arc::new(MockRuleRepo),
            Arc::new(MockInfractionRepo),
            Arc::new(MockCache),
            Arc::new(MockConduct),
            Arc::new(MockIaConfigRepo),
            Arc::new(InferenceRateLimiter::new(4, 0)),
        ).with_text_inference(inference, tokenizer);
    }

    // ══════════════════════════════════════════════════════════
    //  Tests score_classifications — fonction pure, pas de mock
    // ══════════════════════════════════════════════════════════

    // ── Messages neutres ──

    #[test]
    fn neutral_message_returns_none() {
        let classifications = vec![
            cls("neutral", 0.95),
            cls("anger", 0.02),
            cls("rage", 0.01),
            cls("threat", 0.01),
            cls("harassment", 0.01),
        ];
        assert!(score_classifications(&classifications, &[], 0.5).is_none());
    }

    #[test]
    fn all_below_threshold_returns_none() {
        let classifications = vec![
            cls("neutral", 0.30),
            cls("anger", 0.45),
            cls("rage", 0.10),
            cls("threat", 0.10),
            cls("harassment", 0.05),
        ];
        assert!(score_classifications(&classifications, &[], 0.5).is_none());
    }

    #[test]
    fn empty_classifications_returns_none() {
        assert!(score_classifications(&[], &[], 0.5).is_none());
    }

    // ── Détection anger ──

    #[test]
    fn anger_above_threshold_detected() {
        let classifications = vec![
            cls("neutral", 0.20),
            cls("anger", 0.70),
            cls("rage", 0.05),
            cls("threat", 0.03),
            cls("harassment", 0.02),
        ];
        let result = score_classifications(&classifications, &[], 0.5);
        assert!(result.is_some());

        let (score, flags, reason) = result.unwrap();
        assert_eq!(flags, vec![FlagType::Anger]);
        // anger weight=3.0, confidence=0.7 → 3.0 * 0.7 = 2.1
        assert!((score - 2.1).abs() < 0.01);
        assert!(reason.contains("anger"));
        assert!(reason.contains("70%"));
    }

    // ── Détection rage ──

    #[test]
    fn rage_above_threshold_detected() {
        let classifications = vec![
            cls("neutral", 0.05),
            cls("anger", 0.10),
            cls("rage", 0.80),
            cls("threat", 0.03),
            cls("harassment", 0.02),
        ];
        let (score, flags, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(flags, vec![FlagType::Rage]);
        // rage weight=6.0, confidence=0.8 → 4.8
        assert!((score - 4.8).abs() < 0.01);
    }

    // ── Détection threat ──

    #[test]
    fn threat_above_threshold_detected() {
        let classifications = vec![
            cls("neutral", 0.02),
            cls("anger", 0.03),
            cls("rage", 0.05),
            cls("threat", 0.85),
            cls("harassment", 0.05),
        ];
        let (score, flags, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(flags, vec![FlagType::Threat]);
        // threat weight=8.0, confidence=0.85 → 6.8
        assert!((score - 6.8).abs() < 0.01);
    }

    // ── Détection harassment ──

    #[test]
    fn harassment_above_threshold_detected() {
        let classifications = vec![
            cls("neutral", 0.05),
            cls("anger", 0.05),
            cls("rage", 0.05),
            cls("threat", 0.05),
            cls("harassment", 0.80),
        ];
        let (score, flags, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(flags, vec![FlagType::Harassment]);
        // harassment weight=7.0, confidence=0.8 → 5.6
        assert!((score - 5.6).abs() < 0.01);
    }

    // ── Combinaisons ──

    #[test]
    fn anger_plus_rage_combined_score() {
        let classifications = vec![
            cls("neutral", 0.05),
            cls("anger", 0.60),
            cls("rage", 0.70),
            cls("threat", 0.03),
            cls("harassment", 0.02),
        ];
        let (score, flags, reason) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(flags.len(), 2);
        assert!(flags.contains(&FlagType::Anger));
        assert!(flags.contains(&FlagType::Rage));
        // anger: 3.0*0.6=1.8 + rage: 6.0*0.7=4.2 → 6.0
        assert!((score - 6.0).abs() < 0.01);
        assert!(reason.contains("anger"));
        assert!(reason.contains("rage"));
    }

    #[test]
    fn all_toxic_flags_combined() {
        let classifications = vec![
            cls("neutral", 0.01),
            cls("anger", 0.60),
            cls("rage", 0.70),
            cls("threat", 0.80),
            cls("harassment", 0.90),
        ];
        let (score, flags, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(flags.len(), 4);
        // anger:3*0.6=1.8 + rage:6*0.7=4.2 + threat:8*0.8=6.4 + harassment:7*0.9=6.3 = 18.7
        assert!((score - 18.7).abs() < 0.01);
    }

    // ── Seuils personnalisés ──

    #[test]
    fn strict_threshold_filters_out_low_confidence() {
        let classifications = vec![
            cls("anger", 0.70),
            cls("rage", 0.85),
        ];
        // Seuil strict = 0.8 → anger(0.7) rejeté, rage(0.85) accepté
        let (score, flags, _) = score_classifications(&classifications, &[], 0.8).unwrap();
        assert_eq!(flags, vec![FlagType::Rage]);
        // rage: 6.0 * 0.85 = 5.1
        assert!((score - 5.1).abs() < 0.01);
    }

    #[test]
    fn very_strict_threshold_rejects_all() {
        let classifications = vec![
            cls("anger", 0.70),
            cls("rage", 0.80),
            cls("threat", 0.85),
        ];
        // Seuil = 0.95 → tout rejeté
        assert!(score_classifications(&classifications, &[], 0.95).is_none());
    }

    #[test]
    fn zero_threshold_accepts_everything() {
        let classifications = vec![
            cls("anger", 0.01),
            cls("rage", 0.01),
        ];
        let result = score_classifications(&classifications, &[], 0.0);
        assert!(result.is_some());
        assert_eq!(result.unwrap().1.len(), 2);
    }

    #[test]
    fn exact_threshold_boundary_accepted() {
        let classifications = vec![cls("anger", 0.50)];
        // confidence == threshold → accepté (>=)
        let result = score_classifications(&classifications, &[], 0.5);
        assert!(result.is_some());
    }

    #[test]
    fn just_below_threshold_rejected() {
        let classifications = vec![cls("anger", 0.499)];
        assert!(score_classifications(&classifications, &[], 0.5).is_none());
    }

    // ── Règles custom ──

    #[test]
    fn custom_rule_overrides_default_weight() {
        let classifications = vec![cls("anger", 0.80)];
        let rules = vec![make_rule(FlagType::Anger, 10.0)];
        let (score, _, _) = score_classifications(&classifications, &rules, 0.5).unwrap();
        // custom weight=10.0, confidence=0.8 → 8.0 (vs 2.4 par défaut)
        assert!((score - 8.0).abs() < 0.01);
    }

    #[test]
    fn disabled_rule_uses_default_weight() {
        let classifications = vec![cls("anger", 0.80)];
        let mut rule = make_rule(FlagType::Anger, 10.0);
        rule.enabled = false;
        let (score, _, _) = score_classifications(&classifications, &[rule], 0.5).unwrap();
        // rule disabled → default weight=3.0, confidence=0.8 → 2.4
        assert!((score - 2.4).abs() < 0.01);
    }

    #[test]
    fn custom_rule_for_different_flag_no_effect() {
        let classifications = vec![cls("anger", 0.80)];
        let rules = vec![make_rule(FlagType::Rage, 15.0)];
        let (score, _, _) = score_classifications(&classifications, &rules, 0.5).unwrap();
        // rule est pour Rage, pas Anger → default anger weight=3.0
        assert!((score - 2.4).abs() < 0.01);
    }

    #[test]
    fn multiple_custom_rules_applied() {
        let classifications = vec![
            cls("anger", 0.60),
            cls("threat", 0.70),
        ];
        let rules = vec![
            make_rule(FlagType::Anger, 5.0),
            make_rule(FlagType::Threat, 12.0),
        ];
        let (score, _, _) = score_classifications(&classifications, &rules, 0.5).unwrap();
        // anger: 5.0*0.6=3.0 + threat: 12.0*0.7=8.4 → 11.4
        assert!((score - 11.4).abs() < 0.01);
    }

    // ── Labels non reconnus ignorés ──

    #[test]
    fn unknown_labels_ignored() {
        let classifications = vec![
            cls("neutral", 0.90),
            cls("joy", 0.80),
            cls("sadness", 0.70),
        ];
        assert!(score_classifications(&classifications, &[], 0.5).is_none());
    }

    // ── Format de la raison ──

    #[test]
    fn reason_format_single_flag() {
        let classifications = vec![cls("threat", 0.90)];
        let (_, _, reason) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(reason, "IA sentiment : threat(90%)");
    }

    #[test]
    fn reason_format_multiple_flags() {
        let classifications = vec![
            cls("anger", 0.70),
            cls("harassment", 0.80),
        ];
        let (_, _, reason) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(reason, "IA sentiment : anger(70%), harassment(80%)");
    }

    // ══════════════════════════════════════════════════════════
    //  Tests de scoring combiné → action
    // ══════════════════════════════════════════════════════════

    #[test]
    fn anger_only_triggers_warn() {
        // anger: weight=3.0, confidence=0.8 → score=2.4 >= warn(2.0) mais < delete(4.0)
        let classifications = vec![cls("anger", 0.80)];
        let (score, _, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        let (t_warn, t_delete, _, _) = resolve_thresholds(&[]);
        assert!(score >= t_warn);
        assert!(score < t_delete);
    }

    #[test]
    fn rage_triggers_delete_or_mute() {
        // rage: weight=6.0, confidence=0.85 → score=5.1 >= delete(4.0) mais < mute(6.0)
        let classifications = vec![cls("rage", 0.85)];
        let (score, _, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        let (_, t_delete, t_mute, _) = resolve_thresholds(&[]);
        assert!(score >= t_delete);
        assert!(score < t_mute);
    }

    #[test]
    fn threat_high_confidence_triggers_mute() {
        // threat: weight=8.0, confidence=0.90 → score=7.2 >= mute(6.0) mais < ban(9.0)
        let classifications = vec![cls("threat", 0.90)];
        let (score, _, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        let (_, _, t_mute, t_ban) = resolve_thresholds(&[]);
        assert!(score >= t_mute);
        assert!(score < t_ban);
    }

    #[test]
    fn rage_plus_threat_triggers_ban() {
        // rage:6.0*0.8=4.8 + threat:8.0*0.8=6.4 → 11.2 >= ban(9.0)
        let classifications = vec![
            cls("rage", 0.80),
            cls("threat", 0.80),
        ];
        let (score, _, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        let (_, _, _, t_ban) = resolve_thresholds(&[]);
        assert!(score >= t_ban);
    }

    #[test]
    fn anger_low_confidence_below_warn() {
        // anger: weight=3.0, confidence=0.55 → score=1.65 < warn(2.0)
        let classifications = vec![cls("anger", 0.55)];
        let (score, _, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        let (t_warn, _, _, _) = resolve_thresholds(&[]);
        assert!(score < t_warn);
    }

    // ══════════════════════════════════════════════════════════
    //  Tests avec confidences réalistes (somme ~1.0 via softmax)
    // ══════════════════════════════════════════════════════════

    #[test]
    fn realistic_softmax_angry_message() {
        // Softmax distribution typique d'un message colérique
        let classifications = vec![
            cls("neutral", 0.15),
            cls("anger", 0.55),
            cls("rage", 0.15),
            cls("threat", 0.10),
            cls("harassment", 0.05),
        ];
        let (score, flags, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(flags, vec![FlagType::Anger]);
        // anger: 3.0 * 0.55 = 1.65
        assert!((score - 1.65).abs() < 0.01);
    }

    #[test]
    fn realistic_softmax_threat_message() {
        // Message de menace directe
        let classifications = vec![
            cls("neutral", 0.02),
            cls("anger", 0.08),
            cls("rage", 0.10),
            cls("threat", 0.75),
            cls("harassment", 0.05),
        ];
        let (score, flags, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(flags, vec![FlagType::Threat]);
        // threat: 8.0 * 0.75 = 6.0
        assert!((score - 6.0).abs() < 0.01);
    }

    #[test]
    fn realistic_softmax_harassment_escalation() {
        // Harcèlement avec rage sous-jacente
        let classifications = vec![
            cls("neutral", 0.03),
            cls("anger", 0.07),
            cls("rage", 0.55),
            cls("threat", 0.05),
            cls("harassment", 0.30),
        ];
        // threshold 0.5 → rage(0.55) detecté, harassment(0.30) rejeté
        let (score, flags, _) = score_classifications(&classifications, &[], 0.5).unwrap();
        assert_eq!(flags, vec![FlagType::Rage]);
        // rage: 6.0 * 0.55 = 3.3
        assert!((score - 3.3).abs() < 0.01);
    }

    #[test]
    fn realistic_softmax_harassment_escalation_lower_threshold() {
        let classifications = vec![
            cls("neutral", 0.03),
            cls("anger", 0.07),
            cls("rage", 0.55),
            cls("threat", 0.05),
            cls("harassment", 0.30),
        ];
        // Seuil plus bas (0.25) → rage ET harassment détectés
        let (score, flags, _) = score_classifications(&classifications, &[], 0.25).unwrap();
        assert_eq!(flags.len(), 2);
        // rage: 6.0*0.55=3.3 + harassment: 7.0*0.30=2.1 → 5.4
        assert!((score - 5.4).abs() < 0.01);
    }

    // ══════════════════════════════════════════════════════════
    //  Tests build_contextual_content
    // ══════════════════════════════════════════════════════════

    fn ctx_msg(username: &str, content: &str) -> crate::ports::inbound::ContextMessageEntry {
        crate::ports::inbound::ContextMessageEntry {
            username: username.to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn context_empty_returns_content_only() {
        let result = build_contextual_content("hello", &[], "natural");
        assert_eq!(result, "hello");
    }

    #[test]
    fn context_natural_format() {
        let ctx = vec![ctx_msg("Alice", "salut"), ctx_msg("Bob", "ca va ?")];
        let result = build_contextual_content("oui bien", &ctx, "natural");
        assert!(result.contains("Alice: salut"));
        assert!(result.contains("Bob: ca va ?"));
        assert!(result.contains("---"));
        assert!(result.ends_with("oui bien"));
    }

    #[test]
    fn context_tagged_format() {
        let ctx = vec![ctx_msg("Alice", "salut")];
        let result = build_contextual_content("oui", &ctx, "tagged");
        assert!(result.starts_with("[message] oui [/message]"));
        assert!(result.contains("[context] Alice: salut [/context]"));
    }

    #[test]
    fn context_unknown_format_defaults_to_natural() {
        let ctx = vec![ctx_msg("X", "y")];
        let result = build_contextual_content("z", &ctx, "unknown");
        assert!(result.contains("---")); // natural format
    }
}

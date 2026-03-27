use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

use crate::domain::entities::{Infraction, MessageAnalysis};
use crate::domain::errors::DomainError;
use crate::domain::services::{InferenceRateLimiter, InferenceService, ScoringService, TextTokenizer};
use crate::domain::value_objects::{Action, FlagType};
use crate::ports::inbound::{AnalyzeMessageCommand, AnalyzeMessageUseCase, DeductPointsCommand, ManageConductUseCase};
use crate::domain::entities::IaConfig;
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
                self.cache.set_rules(&cmd.guild_id, &from_db).await.ok();
                from_db
            }
        };

        // 2. Scoring basique (flags bot : spam, insult, link, phishing)
        let mut result = ScoringService::score(&cmd.flags, &rules);

        // 3. Inference text IA (sentiment : anger, rage, threat, harassment)
        // Charger la config IA per-guild pour le seuil de confiance
        let ia_config = self.ia_config_repo.get(&cmd.guild_id).await.ok().flatten();
        let text_enabled = ia_config.as_ref().map(|c| c.text_enabled).unwrap_or(true);
        let text_threshold = ia_config.as_ref().map(|c| c.text_threshold as f32).unwrap_or(DEFAULT_TEXT_THRESHOLD);

        if let (Some(inference), Some(tokenizer)) = (&self.inference, &self.tokenizer) {
            if text_enabled && inference.text_available() && tokenizer.available() && !cmd.content.is_empty() {
                // Rate limit inference
                let _permit = self.inference_limiter.acquire().await?;

                match self.run_text_inference(inference, tokenizer, &cmd.content, &rules, text_threshold) {
                    Ok(Some((ia_score, _ia_flags, ia_reason))) => {
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
                            (Action::Mute, Some(600))
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
            let _ = self.conduct_uc.deduct_points(DeductPointsCommand {
                guild_id: infraction.guild_id.clone(),
                user_id: infraction.user_id.clone(),
                username: infraction.username.clone(),
                action: result.action.as_str().to_string(),
            }).await;
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

impl AnalyzeMessageService {
    /// Execute l'inference text et retourne (score_ia, flags_detectes, raison).
    /// Retourne None si aucun sentiment toxique n'est detecte.
    fn run_text_inference(
        &self,
        inference: &InferenceService,
        tokenizer: &TextTokenizer,
        content: &str,
        rules: &[crate::domain::entities::Rule],
        threshold: f32,
    ) -> Result<Option<(f64, Vec<FlagType>, String)>, String> {
        // Tokeniser
        let (input_ids, attention_mask) = tokenizer.tokenize(content)?;

        // Inference
        let classifications = inference.classify_text(input_ids, attention_mask)?;

        // Filtrer les sentiments au-dessus du seuil per-guild
        let mut detected: Vec<(FlagType, f32)> = Vec::new();

        for c in &classifications {
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
            return Ok(None);
        }

        // Calculer le score IA
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
            // Ponderer par la confiance du modele
            let weighted = base_weight * (*confidence as f64);
            ia_score += weighted;
            triggered.push(format!("{}({:.0}%)", flag_type.as_str(), confidence * 100.0));
        }

        let reason = format!("IA sentiment : {}", triggered.join(", "));

        Ok(Some((ia_score, detected.into_iter().map(|(f, _)| f).collect(), reason)))
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
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use crate::domain::entities::Rule;

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
        assert_eq!(w, 2.0); // Disabled => defaults
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
        use crate::domain::services::{InferenceService, TextTokenizer};

        // Creer des instances vides (pas de modele charge)
        let inference = Arc::new(InferenceService::new(None, None));
        let tokenizer = Arc::new(TextTokenizer::new(None, 256));

        // Verifier que without inference, les champs sont None
        // On ne peut pas tester directement les champs prives, mais on peut
        // verifier que la construction ne panique pas
        let _service = AnalyzeMessageService::new(
            Arc::new(MockRuleRepo),
            Arc::new(MockInfractionRepo),
            Arc::new(MockCache),
            Arc::new(MockConduct),
        ).with_text_inference(inference, tokenizer);
    }
}

// ── Mocks minimaux pour les tests ──

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

use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use uuid::Uuid;

use crate::domain::entities::moderation::infraction::Infraction;
use crate::domain::entities::ai::message_analysis::MessageAnalysis;
use crate::domain::errors::DomainError;
use crate::ports::outbound::ai::inference_service::InferenceService;
use crate::ports::outbound::ai::text_tokenizer::TextTokenizer;
use crate::domain::services::moderation::channel_tension::ChannelTensionBuffer;
use crate::domain::services::ai::inference_limiter::InferenceRateLimiter;
use crate::domain::services::moderation::scoring_service::ScoringService;
use crate::domain::services::moderation::channel_tension::TensionAction;
use crate::domain::services::moderation::channel_tension::TensionEntry;
use crate::domain::enums::moderation::action::Action;
use crate::domain::enums::moderation::flag_type::FlagType;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use crate::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use crate::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::ports::outbound::system::cache::CachePort;
use crate::ports::outbound::moderation::infraction_repository::InfractionRepository;
use crate::ports::outbound::moderation::rule_repository::RuleRepository;
/// Seuil de confiance par defaut (utilise si pas de config per-guild).
const DEFAULT_TEXT_THRESHOLD: f32 = 0.5;

pub struct AnalyzeMessageService {
    rule_repo: Arc<dyn RuleRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
    /// Repo pour lire la config automod-bot : cles IA (text_enabled,
    /// text_threshold, context_dampening, context_format) + cles tension
    /// de salon (activation + seuils). Anciennement lu depuis la table
    /// dediee `ia_config` ; fusion dans automod-bot via migration 146.
    bot_config_repo: Arc<dyn BotConfigRepository>,
    inference_limiter: Arc<InferenceRateLimiter>,
    inference: Option<Arc<dyn InferenceService>>,
    tokenizer: Option<Arc<dyn TextTokenizer>>,
    /// Buffer in-memory pour la "tension de salon" (option : si None, la
    /// feature est desactivee quel que soit le contenu de la config).
    tension_buffer: Option<Arc<ChannelTensionBuffer>>,
}

impl AnalyzeMessageService {
    pub fn new(
        rule_repo: Arc<dyn RuleRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
        inference_limiter: Arc<InferenceRateLimiter>,
    ) -> Self {
        Self {
            rule_repo,
            infraction_repo,
            cache,
            bot_config_repo,
            inference_limiter,
            inference: None,
            tokenizer: None,
            tension_buffer: None,
        }
    }

    /// Ajoute l'inference text IA au service d'analyse.
    pub fn with_text_inference(
        mut self,
        inference: Arc<dyn InferenceService>,
        tokenizer: Arc<dyn TextTokenizer>,
    ) -> Self {
        self.inference = Some(inference);
        self.tokenizer = Some(tokenizer);
        self
    }

    /// Ajoute la feature "tension de salon" (buffer glissant + seuils
    /// lus depuis `bot_guild_config` pour `automod-bot`).
    pub fn with_channel_tension(mut self, buffer: Arc<ChannelTensionBuffer>) -> Self {
        self.tension_buffer = Some(buffer);
        self
    }
}

/// Config IA resolue depuis la config `automod-bot` (migration 146).
#[derive(Debug, Clone)]
pub(crate) struct IaConfigValues {
    pub text_enabled: bool,
    pub text_threshold: f32,
    pub context_dampening: f64,
    pub context_format: String,
}

impl Default for IaConfigValues {
    fn default() -> Self {
        Self {
            text_enabled: true,
            text_threshold: DEFAULT_TEXT_THRESHOLD,
            context_dampening: 0.65,
            context_format: "natural".to_string(),
        }
    }
}

/// Parse les cles IA (`text_enabled`, `text_threshold`, `context_dampening`,
/// `context_format`) depuis la liste des `BotGuildConfig` de `automod-bot`.
/// Fallback sur les defauts si cles absentes/malformees.
pub(crate) fn parse_ia_config_from_bot_config(
    entries: &[crate::domain::entities::system::bot_config::BotGuildConfig],
) -> IaConfigValues {
    let mut cfg = IaConfigValues::default();
    for e in entries {
        match e.config_key.as_str() {
            "text_enabled" => {
                let v = e.config_value.to_ascii_lowercase();
                cfg.text_enabled = matches!(v.as_str(), "true" | "1" | "yes");
            }
            "text_threshold" => {
                if let Ok(n) = e.config_value.parse::<f32>() {
                    cfg.text_threshold = n.clamp(0.0, 1.0);
                }
            }
            "context_dampening" => {
                if let Ok(n) = e.config_value.parse::<f64>() {
                    cfg.context_dampening = n.clamp(0.0, 1.0);
                }
            }
            "context_format" => {
                let v = e.config_value.as_str();
                if matches!(v, "natural" | "tagged") {
                    cfg.context_format = v.to_string();
                }
            }
            _ => {}
        }
    }
    cfg
}

/// Config resolue pour la feature "tension de salon".
#[derive(Debug, Clone)]
struct TensionConfig {
    enabled: bool,
    buffer_size: usize,
    threshold_warn: f64,
    threshold_delete: f64,
    threshold_mute: f64,
    mute_duration_secs: u64,
}

impl Default for TensionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            buffer_size: 5,
            threshold_warn: 3.0,
            threshold_delete: 5.0,
            threshold_mute: 7.0,
            mute_duration_secs: 300,
        }
    }
}

/// Parse la config tension depuis la liste des `BotGuildConfig` de
/// `automod-bot`. Defaut si cles absentes/mal formees.
fn parse_tension_config(entries: &[crate::domain::entities::system::bot_config::BotGuildConfig]) -> TensionConfig {
    let mut cfg = TensionConfig::default();
    for e in entries {
        match e.config_key.as_str() {
            "channel_tension_enabled" => {
                let v = e.config_value.to_ascii_lowercase();
                cfg.enabled = matches!(v.as_str(), "true" | "1" | "yes");
            }
            "channel_tension_buffer_size" => {
                if let Ok(n) = e.config_value.parse::<usize>() {
                    if n >= 1 {
                        cfg.buffer_size = n;
                    }
                }
            }
            "channel_tension_threshold_warn" => {
                if let Ok(n) = e.config_value.parse::<f64>() { cfg.threshold_warn = n; }
            }
            "channel_tension_threshold_delete" => {
                if let Ok(n) = e.config_value.parse::<f64>() { cfg.threshold_delete = n; }
            }
            "channel_tension_threshold_mute" => {
                if let Ok(n) = e.config_value.parse::<f64>() { cfg.threshold_mute = n; }
            }
            "channel_tension_mute_duration_secs" => {
                if let Ok(n) = e.config_value.parse::<u64>() { cfg.mute_duration_secs = n; }
            }
            _ => {}
        }
    }
    cfg
}

/// Compare la severite d'une action existante et d'une `TensionAction`
/// pour garder la plus forte si les deux declenchent. Retourne `true`
/// si la tension est strictement plus severe.
fn tension_is_stronger(current: &Action, tension: TensionAction) -> bool {
    let sev = |a: &Action| -> u8 {
        match a {
            Action::None => 0,
            Action::Warn => 1,
            Action::Delete => 2,
            Action::Mute => 3,
            Action::Ban => 4,
        }
    };
    let tsev = match tension {
        TensionAction::None => 0,
        TensionAction::Warn => 1,
        TensionAction::Delete => 2,
        TensionAction::Mute => 3,
    };
    tsev > sev(current)
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
        // Score IA individuel de CE message (0.0 si pas d'inference ou non
        // toxique). Alimente le buffer "tension de salon" apres l'inference.
        let mut ia_score_individual: f64 = 0.0;

        // 3. Inference text IA (sentiment : anger, rage, threat, harassment)
        // Charger la config automod-bot (fusionnee avec l'ancien `ia_config`
        // par la migration 146). On recupere toutes les cles une fois pour
        // partager la lecture avec le bloc "tension de salon" plus bas.
        let automod_entries = match self.bot_config_repo.get_config(&cmd.guild_id, "automod-bot").await {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %cmd.guild_id, "Echec lecture config automod-bot, utilisation defauts");
                vec![]
            }
        };
        let ia_cfg = parse_ia_config_from_bot_config(&automod_entries);
        let text_enabled = ia_cfg.text_enabled;
        let text_threshold = ia_cfg.text_threshold;
        let context_dampening = ia_cfg.context_dampening;
        let context_format = ia_cfg.context_format.clone();
        // Duree de mute configurable (defaut 600s = 10 min).
        // Pas dans la config IA, lu depuis scoring ou defaut.
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
                        ia_score_individual = ia_score;
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

        // 3b. Tension de salon (somme glissante des scores IA des N derniers
        // messages du channel). S'ajoute comme second declencheur : si la
        // tension declenche une action plus severe que l'analyse individuelle,
        // on override. Sinon, l'action individuelle est gardee.
        if let Some(buffer) = self.tension_buffer.as_ref() {
            let tcfg = parse_tension_config(&automod_entries);
            if tcfg.enabled {
                let entry = TensionEntry {
                    score: ia_score_individual,
                    user_id: cmd.user_id.clone(),
                    message_id: cmd.message_id.clone(),
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                };
                let total = buffer.push_and_sum(
                    &cmd.guild_id,
                    &cmd.channel_id,
                    entry,
                    tcfg.buffer_size,
                );
                let action = ChannelTensionBuffer::decide_action(
                    total,
                    tcfg.threshold_warn,
                    tcfg.threshold_delete,
                    tcfg.threshold_mute,
                );
                if action != TensionAction::None {
                    info!(
                        guild_id = %cmd.guild_id,
                        channel_id = %cmd.channel_id,
                        tension_total = total,
                        tension_action = ?action,
                        "Tension de salon declenchee"
                    );
                    if tension_is_stronger(&result.action, action) {
                        let (new_action, duration) = match action {
                            TensionAction::Mute => (Action::Mute, Some(tcfg.mute_duration_secs)),
                            TensionAction::Delete => (Action::Delete, None),
                            TensionAction::Warn => (Action::Warn, None),
                            TensionAction::None => (Action::None, None),
                        };
                        let tension_reason = format!(
                            "Tension de salon (somme glissante {:.2} sur {} derniers messages)",
                            total, tcfg.buffer_size
                        );
                        result.reason = if result.reason.is_empty() {
                            tension_reason
                        } else {
                            format!("{} + {}", result.reason, tension_reason)
                        };
                        result.action = new_action;
                        result.duration = duration;
                    }
                    // Vider le buffer apres declenchement pour eviter le
                    // re-trigger immediat au message suivant (laisse la
                    // conversation redescendre).
                    buffer.clear_channel(&cmd.guild_id, &cmd.channel_id);
                }
            }
        }

        // 3bis. Decision de routage (DECIDE = API) : on connait ici la config
        // guild + le score + les flags. Le bot n'aura qu'a EXECUTER.
        let routing = {
            use crate::domain::services::moderation::automod_routing::{decide, RoutingInputs};
            let cfg_str = |k: &str| {
                automod_entries
                    .iter()
                    .find(|e| e.config_key == k)
                    .map(|e| e.config_value.as_str())
            };
            let cfg_bool = |k: &str, d: bool| {
                cfg_str(k).map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes")).unwrap_or(d)
            };
            let cfg_f64 = |k: &str, d: f64| cfg_str(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(d);
            let cfg_u64 = |k: &str, d: u64| cfg_str(k).and_then(|v| v.parse::<u64>().ok()).unwrap_or(d);
            decide(&RoutingInputs {
                flags: &cmd.flags,
                content: &cmd.content,
                score: result.score,
                action: result.action.clone(),
                human_only: cfg_bool("human_only_enabled", false),
                auto_protect: cfg_bool("auto_protect_enabled", true),
                auto_delete_links: cfg_bool("auto_delete_links_enabled", true),
                ai_review_mode: cfg_bool("ai_review_mode", true),
                review_min_score: cfg_f64("review_min_score", 0.0),
                log_channel_set: cfg_u64("log_channel_id", 0) != 0,
            })
        };

        // 4. Persister l'infraction
        let infraction = Infraction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            channel_id: cmd.channel_id,
            user_id: cmd.user_id,
            username: cmd.username,
            display_name: None,
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

        // 5. Retourner l'analyse + la decision de routage
        Ok(MessageAnalysis {
            action: result.action,
            reason: result.reason,
            score: result.score,
            duration: result.duration,
            route: routing.route,
            severe: routing.severe,
            auto_delete_link: routing.auto_delete_link,
        })
    }
}

// run_text_inference supprimee — remplacee par spawn_blocking + timeout dans analyze().

/// Fonction pure : transforme les classifications IA en score, flags et raison.
/// Retourne None si aucun sentiment toxique n'est detecte au-dessus du seuil.
pub fn score_classifications(
    classifications: &[crate::ports::outbound::ai::inference_service::InferenceClassification],
    rules: &[crate::domain::entities::system::rule::Rule],
    threshold: f32,
) -> Option<(f64, Vec<FlagType>, String)> {
    let mut detected: Vec<(FlagType, f32)> = Vec::new();

    for c in classifications {
        let flag = match c.label.as_str() {
            // Modele 2 classes : severe = rage + threat agreges.
            // On mappe sur FlagType::Harassment (la plus generique des flags
            // toxiques) pour que le scoring existant fonctionne sans ajouter
            // un nouveau type.
            "severe" if c.confidence >= threshold => Some(FlagType::Harassment),
            // Legacy 5 classes (si vieux modele encore charge).
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
    context: &[crate::ports::inbound::ai::analyze_message::ContextMessageEntry],
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

fn resolve_thresholds(rules: &[crate::domain::entities::system::rule::Rule]) -> (f64, f64, f64, f64) {
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

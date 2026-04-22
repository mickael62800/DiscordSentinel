use super::*;
use crate::ports::outbound::BotConfigRepository;


struct MockBotConfigRepo;


#[async_trait]
impl BotConfigRepository for MockBotConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<crate::domain::entities::BotDefinition>, crate::domain::errors::DomainError> { Ok(vec![]) }
    async fn get_config(&self, _: &str, _: &str) -> Result<Vec<crate::domain::entities::BotGuildConfig>, crate::domain::errors::DomainError> { Ok(vec![]) }
    async fn get_all_config(&self, _: &str) -> Result<Vec<crate::domain::entities::BotGuildConfig>, crate::domain::errors::DomainError> { Ok(vec![]) }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
}


struct MockRuleRepo;


#[async_trait]
impl RuleRepository for MockRuleRepo {
    async fn find_by_guild(&self, _: &str) -> Result<Vec<crate::domain::entities::Rule>, crate::domain::errors::DomainError> { Ok(vec![]) }
    async fn find_all(&self) -> Result<Vec<crate::domain::entities::Rule>, crate::domain::errors::DomainError> { Ok(vec![]) }
    async fn find_by_id(&self, _: uuid::Uuid) -> Result<Option<crate::domain::entities::Rule>, crate::domain::errors::DomainError> { Ok(None) }
    async fn save(&self, rule: &crate::domain::entities::Rule) -> Result<crate::domain::entities::Rule, crate::domain::errors::DomainError> { Ok(rule.clone()) }
    async fn toggle(&self, _: uuid::Uuid, _: bool) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
    async fn delete(&self, _: uuid::Uuid) -> Result<(), crate::domain::errors::DomainError> { Ok(()) }
}


struct MockInfractionRepo;


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


struct MockCache;


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


struct MockConduct;


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


    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use crate::domain::entities::Rule;
    use crate::adapters::outbound::InferenceClassification;

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
        use crate::adapters::outbound::{InferenceService, TextTokenizer};
        use crate::domain::services::InferenceRateLimiter;

        let inference = Arc::new(InferenceService::new(None, None));
        let tokenizer = Arc::new(TextTokenizer::new(None, 256));

        let _service = AnalyzeMessageService::new(
            Arc::new(MockRuleRepo),
            Arc::new(MockInfractionRepo),
            Arc::new(MockCache),
            Arc::new(MockConduct),
            Arc::new(MockBotConfigRepo),
            Arc::new(InferenceRateLimiter::new(4, 0)),
        ).with_text_inference(inference, tokenizer);
    }

    // ══════════════════════════════════════════════════════════
    //  Tests parse_ia_config_from_bot_config
    // ══════════════════════════════════════════════════════════

    fn bot_entry(key: &str, value: &str) -> crate::domain::entities::BotGuildConfig {
        crate::domain::entities::BotGuildConfig {
            id: uuid::Uuid::new_v4(),
            guild_id: "g".to_string(),
            bot_name: "automod-bot".to_string(),
            config_key: key.to_string(),
            config_value: value.to_string(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn parse_ia_config_empty_returns_defaults() {
        let cfg = parse_ia_config_from_bot_config(&[]);
        assert!(cfg.text_enabled);
        assert!((cfg.text_threshold - 0.5).abs() < 1e-6);
        assert!((cfg.context_dampening - 0.65).abs() < 1e-6);
        assert_eq!(cfg.context_format, "natural");
    }

    #[test]
    fn parse_ia_config_reads_all_keys() {
        let entries = vec![
            bot_entry("text_enabled", "false"),
            bot_entry("text_threshold", "0.8"),
            bot_entry("context_dampening", "0.3"),
            bot_entry("context_format", "tagged"),
        ];
        let cfg = parse_ia_config_from_bot_config(&entries);
        assert!(!cfg.text_enabled);
        assert!((cfg.text_threshold - 0.8).abs() < 1e-6);
        assert!((cfg.context_dampening - 0.3).abs() < 1e-6);
        assert_eq!(cfg.context_format, "tagged");
    }

    #[test]
    fn parse_ia_config_clamps_out_of_range() {
        let entries = vec![
            bot_entry("text_threshold", "5.0"),
            bot_entry("context_dampening", "-1.0"),
        ];
        let cfg = parse_ia_config_from_bot_config(&entries);
        assert!((cfg.text_threshold - 1.0).abs() < 1e-6);
        assert!((cfg.context_dampening - 0.0).abs() < 1e-6);
    }

    #[test]
    fn parse_ia_config_ignores_invalid_values_and_format() {
        let entries = vec![
            bot_entry("text_threshold", "not-a-number"),
            bot_entry("context_dampening", "abc"),
            bot_entry("context_format", "unknown"),
            bot_entry("text_enabled", "yes"),
        ];
        let cfg = parse_ia_config_from_bot_config(&entries);
        // Les cles invalides retombent sur defaut
        assert!((cfg.text_threshold - 0.5).abs() < 1e-6);
        assert!((cfg.context_dampening - 0.65).abs() < 1e-6);
        assert_eq!(cfg.context_format, "natural");
        assert!(cfg.text_enabled); // "yes" reconnu
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

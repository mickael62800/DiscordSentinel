use super::*;
use crate::domain::entities::system::rule::Rule;
use chrono::Utc;
use uuid::Uuid;

fn make_flags(spam: bool, insult: bool, link: bool) -> DetectionFlags {
    DetectionFlags {
        spam,
        insult,
        profanity: false,
        link,
        phishing: false,
    }
}

fn make_rule(flag_type: FlagType, weight: f64) -> Rule {
    let now = Utc::now();
    Rule {
        id: Uuid::new_v4(),
        guild_id: "test".into(),
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
fn test_no_flags_returns_none() {
    let result = ScoringService::score(&make_flags(false, false, false), &[]);
    assert_eq!(result.action, Action::None);
    assert_eq!(result.score, 0.0);
}

#[test]
fn test_insult_default_triggers_delete() {
    let result = ScoringService::score(&make_flags(false, true, false), &[]);
    assert_eq!(result.action, Action::Delete);
    assert_eq!(result.score, 5.0);
}

#[test]
fn test_spam_default_triggers_warn() {
    let result = ScoringService::score(&make_flags(true, false, false), &[]);
    assert_eq!(result.action, Action::Warn);
    assert_eq!(result.score, 3.0);
}

#[test]
fn test_link_default_below_warn() {
    let result = ScoringService::score(&make_flags(false, false, true), &[]);
    assert_eq!(result.action, Action::None);
    assert_eq!(result.score, 1.0);
}

#[test]
fn test_spam_plus_insult_triggers_mute() {
    let result = ScoringService::score(&make_flags(true, true, false), &[]);
    assert_eq!(result.action, Action::Mute);
    assert_eq!(result.score, 8.0);
    assert_eq!(result.duration, Some(600));
}

#[test]
fn test_all_flags_triggers_ban() {
    let result = ScoringService::score(&make_flags(true, true, true), &[]);
    assert_eq!(result.action, Action::Ban);
    assert_eq!(result.score, 9.0);
}

#[test]
fn test_custom_rules_override_weights() {
    let rules = vec![make_rule(FlagType::Insult, 2.0)];
    let result = ScoringService::score(&make_flags(false, true, false), &rules);
    assert_eq!(result.score, 2.0);
    assert_eq!(result.action, Action::Warn);
}

#[test]
fn test_disabled_rule_uses_default() {
    let mut rule = make_rule(FlagType::Insult, 0.5);
    rule.enabled = false;
    let result = ScoringService::score(&make_flags(false, true, false), &[rule]);
    assert_eq!(result.score, 5.0);
}

#[test]
fn test_phishing_default_triggers_mute() {
    let flags = DetectionFlags {
        spam: false,
        insult: false,
        profanity: false,
        link: false,
        phishing: true,
    };
    let result = ScoringService::score(&flags, &[]);
    assert_eq!(result.action, Action::Mute);
    assert_eq!(result.score, 7.0);
}

#[test]
fn test_phishing_plus_spam_triggers_ban() {
    let flags = DetectionFlags {
        spam: true,
        insult: false,
        profanity: false,
        link: false,
        phishing: true,
    };
    let result = ScoringService::score(&flags, &[]);
    assert_eq!(result.action, Action::Ban);
    assert_eq!(result.score, 10.0);
}

#[test]
fn test_reason_contains_flags() {
    let result = ScoringService::score(&make_flags(true, true, false), &[]);
    assert!(result.reason.contains("spam"));
    assert!(result.reason.contains("insult"));
}

#[test]
fn test_nsfw_default_weight() {
    assert_eq!(default_weight(&FlagType::Nsfw), 8.0);
}

#[test]
fn test_illicit_default_weight() {
    assert_eq!(default_weight(&FlagType::Illicit), 9.0);
}

#[test]
fn test_anger_default_weight() {
    assert_eq!(default_weight(&FlagType::Anger), 3.0);
}

#[test]
fn test_rage_default_weight() {
    assert_eq!(default_weight(&FlagType::Rage), 6.0);
}

#[test]
fn test_threat_default_weight() {
    assert_eq!(default_weight(&FlagType::Threat), 8.0);
}

#[test]
fn test_harassment_default_weight() {
    assert_eq!(default_weight(&FlagType::Harassment), 7.0);
}

#[test]
fn test_custom_nsfw_rule_overrides_weight() {
    let rules = [make_rule(FlagType::Nsfw, 4.0)];
    let rule = rules
        .iter()
        .find(|r| r.flag_type == FlagType::Nsfw && r.enabled);
    assert_eq!(rule.unwrap().weight, 4.0);
}

#[test]
fn test_resolve_thresholds_empty_rules() {
    let (w, d, m, b) = resolve_thresholds(&[], &[FlagType::Spam], &ScoringConfig::default());
    assert_eq!(w, DEFAULT_THRESHOLD_WARN);
    assert_eq!(d, DEFAULT_THRESHOLD_DELETE);
    assert_eq!(m, DEFAULT_THRESHOLD_MUTE);
    assert_eq!(b, DEFAULT_THRESHOLD_BAN);
}

#[test]
fn test_resolve_thresholds_takes_strictest() {
    let mut rule1 = make_rule(FlagType::Spam, 3.0);
    rule1.threshold_warn = 1.0;
    rule1.threshold_ban = 8.0;

    let mut rule2 = make_rule(FlagType::Insult, 5.0);
    rule2.threshold_warn = 3.0;
    rule2.threshold_ban = 10.0;

    // Les deux flags sont déclenchés -> on prend le seuil le plus strict.
    let (w, _, _, b) = resolve_thresholds(
        &[rule1, rule2],
        &[FlagType::Spam, FlagType::Insult],
        &ScoringConfig::default(),
    );
    assert_eq!(w, 1.0);
    assert_eq!(b, 8.0);
}

#[test]
fn test_resolve_thresholds_ignores_unrelated_flag() {
    // Règle stricte sur les LIENS (seuils très bas) + règle insulte normale.
    let mut strict_link = make_rule(FlagType::Link, 1.0);
    strict_link.threshold_warn = 0.5;
    strict_link.threshold_delete = 1.0;
    strict_link.threshold_mute = 1.5;
    strict_link.threshold_ban = 2.0;

    let insult = make_rule(FlagType::Insult, 5.0);

    // Détection d'INSULTE uniquement : la règle stricte sur les liens ne doit
    // PAS abaisser les seuils -> on garde ceux de la règle insulte (defaults).
    let (w, d, m, b) = resolve_thresholds(
        &[strict_link, insult],
        &[FlagType::Insult],
        &ScoringConfig::default(),
    );
    assert_eq!(w, 2.0);
    assert_eq!(d, 4.0);
    assert_eq!(m, 6.0);
    assert_eq!(b, 9.0);
}

#[test]
fn test_resolve_thresholds_no_matching_rule_uses_defaults() {
    // Une règle stricte existe mais sur un flag non déclenché -> defaults.
    let mut strict_link = make_rule(FlagType::Link, 1.0);
    strict_link.threshold_warn = 0.5;
    let (w, _, _, _) = resolve_thresholds(
        &[strict_link],
        &[FlagType::Insult],
        &ScoringConfig::default(),
    );
    assert_eq!(w, DEFAULT_THRESHOLD_WARN);
}

#[test]
fn test_strict_link_rule_does_not_lower_insult_action() {
    // Régression C4 : une règle stricte sur les liens ne doit pas faire passer
    // une simple détection d'insulte (poids 5, défaut) en Mute/Ban.
    let mut strict_link = make_rule(FlagType::Link, 1.0);
    strict_link.threshold_warn = 0.5;
    strict_link.threshold_delete = 0.8;
    strict_link.threshold_mute = 1.0;
    strict_link.threshold_ban = 1.5;

    let result = ScoringService::score(&make_flags(false, true, false), &[strict_link]);
    // Insulte = 5.0 ; seuils insulte par défaut -> Delete (4.0), pas Ban.
    assert_eq!(result.score, 5.0);
    assert_eq!(result.action, Action::Delete);
}

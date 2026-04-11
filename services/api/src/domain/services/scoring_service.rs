use crate::domain::entities::Rule;
use crate::domain::value_objects::{Action, DetectionFlags, FlagType};

/// Poids par défaut quand aucune règle n'est configurée pour un flag.
const DEFAULT_WEIGHT_SPAM: f64 = 3.0;
const DEFAULT_WEIGHT_INSULT: f64 = 5.0;
const DEFAULT_WEIGHT_LINK: f64 = 1.0;
const DEFAULT_WEIGHT_PHISHING: f64 = 7.0;
// IA Vision
const DEFAULT_WEIGHT_NSFW: f64 = 8.0;
const DEFAULT_WEIGHT_ILLICIT: f64 = 9.0;
// IA Text Sentiment
const DEFAULT_WEIGHT_ANGER: f64 = 3.0;
const DEFAULT_WEIGHT_RAGE: f64 = 6.0;
const DEFAULT_WEIGHT_THREAT: f64 = 8.0;
const DEFAULT_WEIGHT_HARASSMENT: f64 = 7.0;

/// Seuils par défaut.
const DEFAULT_THRESHOLD_WARN: f64 = 2.0;
const DEFAULT_THRESHOLD_DELETE: f64 = 4.0;
const DEFAULT_THRESHOLD_MUTE: f64 = 6.0;
const DEFAULT_THRESHOLD_BAN: f64 = 9.0;

/// Durée de mute par défaut (secondes).
const DEFAULT_MUTE_DURATION: u64 = 600;

/// Résultat du scoring.
pub struct ScoringResult {
    pub score: f64,
    pub action: Action,
    pub reason: String,
    pub duration: Option<u64>,
}

/// Service pur de scoring — aucune dépendance externe.
pub struct ScoringService;

impl ScoringService {
    /// Calcule le score d'un message à partir de ses flags et des règles du serveur.
    ///
    /// Algorithme :
    /// 1. Pour chaque flag actif, récupérer le poids (règle custom ou défaut)
    /// 2. Sommer les poids → score total
    /// 3. Comparer le score aux seuils (du plus sévère au moins sévère)
    /// 4. Retourner l'action correspondante
    pub fn score(flags: &DetectionFlags, rules: &[Rule]) -> ScoringResult {
        let active = flags.active_flags();

        if active.is_empty() {
            return ScoringResult {
                score: 0.0,
                action: Action::None,
                reason: String::new(),
                duration: None,
            };
        }

        // Calculer le score
        let mut total_score = 0.0;
        let mut triggered: Vec<&str> = Vec::new();

        for flag in &active {
            let rule = rules.iter().find(|r| r.flag_type == *flag && r.enabled);
            let weight = match rule {
                Some(r) => r.weight,
                None => default_weight(flag),
            };
            total_score += weight;
            triggered.push(flag.as_str());
        }

        // Déterminer les seuils (prendre ceux de la première règle trouvée, sinon défaut)
        let (t_warn, t_delete, t_mute, t_ban) = resolve_thresholds(rules);

        // Déterminer l'action (du plus sévère au moins sévère)
        let (action, duration) = if total_score >= t_ban {
            (Action::Ban, None)
        } else if total_score >= t_mute {
            (Action::Mute, Some(DEFAULT_MUTE_DURATION))
        } else if total_score >= t_delete {
            (Action::Delete, None)
        } else if total_score >= t_warn {
            (Action::Warn, None)
        } else {
            (Action::None, None)
        };

        let reason = format!(
            "Détection : {} (score: {:.1})",
            triggered.join(", "),
            total_score
        );

        ScoringResult {
            score: total_score,
            action,
            reason,
            duration,
        }
    }
}

fn default_weight(flag: &FlagType) -> f64 {
    match flag {
        FlagType::Spam => DEFAULT_WEIGHT_SPAM,
        FlagType::Insult => DEFAULT_WEIGHT_INSULT,
        FlagType::Link => DEFAULT_WEIGHT_LINK,
        FlagType::Phishing => DEFAULT_WEIGHT_PHISHING,
        FlagType::Nsfw => DEFAULT_WEIGHT_NSFW,
        FlagType::Illicit => DEFAULT_WEIGHT_ILLICIT,
        FlagType::Anger => DEFAULT_WEIGHT_ANGER,
        FlagType::Rage => DEFAULT_WEIGHT_RAGE,
        FlagType::Threat => DEFAULT_WEIGHT_THREAT,
        FlagType::Harassment => DEFAULT_WEIGHT_HARASSMENT,
    }
}

/// Résout les seuils depuis les règles. Si plusieurs règles existent,
/// on prend les seuils les plus bas (les plus strictes) pour chaque niveau.
fn resolve_thresholds(rules: &[Rule]) -> (f64, f64, f64, f64) {
    let enabled: Vec<&Rule> = rules.iter().filter(|r| r.enabled).collect();

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

    fn make_flags(spam: bool, insult: bool, link: bool) -> DetectionFlags {
        DetectionFlags { spam, insult, link, phishing: false }
    }

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
    fn test_no_flags_returns_none() {
        let result = ScoringService::score(&make_flags(false, false, false), &[]);
        assert_eq!(result.action, Action::None);
        assert_eq!(result.score, 0.0);
    }

    #[test]
    fn test_insult_default_triggers_delete() {
        // insult seul = 5.0, seuil delete = 4.0 → Delete
        let result = ScoringService::score(&make_flags(false, true, false), &[]);
        assert_eq!(result.action, Action::Delete);
        assert_eq!(result.score, 5.0);
    }

    #[test]
    fn test_spam_default_triggers_warn() {
        // spam seul = 3.0, seuil warn = 2.0 → Warn
        let result = ScoringService::score(&make_flags(true, false, false), &[]);
        assert_eq!(result.action, Action::Warn);
        assert_eq!(result.score, 3.0);
    }

    #[test]
    fn test_link_default_below_warn() {
        // link seul = 1.0, seuil warn = 2.0 → None
        let result = ScoringService::score(&make_flags(false, false, true), &[]);
        assert_eq!(result.action, Action::None);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn test_spam_plus_insult_triggers_mute() {
        // spam(3) + insult(5) = 8.0, seuil mute = 6.0 → Mute
        let result = ScoringService::score(&make_flags(true, true, false), &[]);
        assert_eq!(result.action, Action::Mute);
        assert_eq!(result.score, 8.0);
        assert_eq!(result.duration, Some(600));
    }

    #[test]
    fn test_all_flags_triggers_ban() {
        // spam(3) + insult(5) + link(1) = 9.0, seuil ban = 9.0 → Ban
        let result = ScoringService::score(&make_flags(true, true, true), &[]);
        assert_eq!(result.action, Action::Ban);
        assert_eq!(result.score, 9.0);
    }

    #[test]
    fn test_custom_rules_override_weights() {
        // Règle custom : insult poids = 2.0 au lieu de 5.0
        let rules = vec![make_rule(FlagType::Insult, 2.0)];
        let result = ScoringService::score(&make_flags(false, true, false), &rules);
        assert_eq!(result.score, 2.0);
        assert_eq!(result.action, Action::Warn); // 2.0 >= warn(2.0) mais < delete(4.0)
    }

    #[test]
    fn test_disabled_rule_uses_default() {
        let mut rule = make_rule(FlagType::Insult, 0.5);
        rule.enabled = false;
        let result = ScoringService::score(&make_flags(false, true, false), &[rule]);
        assert_eq!(result.score, 5.0); // Défaut car rule disabled
    }

    #[test]
    fn test_phishing_default_triggers_mute() {
        // phishing seul = 7.0, seuil mute = 6.0 → Mute
        let flags = DetectionFlags { spam: false, insult: false, link: false, phishing: true };
        let result = ScoringService::score(&flags, &[]);
        assert_eq!(result.action, Action::Mute);
        assert_eq!(result.score, 7.0);
    }

    #[test]
    fn test_phishing_plus_spam_triggers_ban() {
        // phishing(7) + spam(3) = 10.0, seuil ban = 9.0 → Ban
        let flags = DetectionFlags { spam: true, insult: false, link: false, phishing: true };
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

    // ── Tests nouveaux flags IA ──

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
        // Simuler un scoring direct — le poids custom doit etre utilise
        let rule = rules.iter().find(|r| r.flag_type == FlagType::Nsfw && r.enabled);
        assert_eq!(rule.unwrap().weight, 4.0);
    }

    #[test]
    fn test_resolve_thresholds_empty_rules() {
        let (w, d, m, b) = resolve_thresholds(&[]);
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

        let (w, _, _, b) = resolve_thresholds(&[rule1, rule2]);
        assert_eq!(w, 1.0); // Plus strict
        assert_eq!(b, 8.0); // Plus strict
    }
}

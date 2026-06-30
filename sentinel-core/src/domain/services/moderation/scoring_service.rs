use crate::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::domain::entities::system::rule::Rule;
use crate::domain::enums::moderation::action::Action;
use crate::domain::enums::moderation::flag_type::FlagType;
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
        Self::score_with_mute_duration(flags, rules, DEFAULT_MUTE_DURATION)
    }

    /// Version paramétrique avec durée de mute configurable.
    pub fn score_with_mute_duration(
        flags: &DetectionFlags,
        rules: &[Rule],
        mute_duration: u64,
    ) -> ScoringResult {
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

        // Déterminer les seuils à partir des SEULES règles dont le flag a été
        // déclenché (per-flag-type) : une règle stricte sur une catégorie sans
        // rapport (ex. liens) ne doit pas abaisser le seuil d'une autre (ex. insulte).
        let (t_warn, t_delete, t_mute, t_ban) = resolve_thresholds(rules, &active);

        // Déterminer l'action (du plus sévère au moins sévère)
        let (action, duration) = if total_score >= t_ban {
            (Action::Ban, None)
        } else if total_score >= t_mute {
            (Action::Mute, Some(mute_duration))
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

/// Résout les seuils depuis les règles, en ne considérant QUE les règles dont
/// le `flag_type` figure parmi les flags réellement déclenchés (`fired`).
///
/// Motivation (correctness) : le score somme les poids des flags déclenchés ;
/// les seuils doivent donc venir des mêmes catégories. Avant, on prenait le
/// minimum des seuils sur TOUTES les règles activées, si bien qu'une règle très
/// stricte (seuil bas) sur une catégorie sans rapport abaissait le seuil de
/// toutes les autres détections. On restreint désormais aux règles pertinentes.
///
/// Comportement : parmi les règles activées dont le flag est déclenché, on
/// prend le seuil le plus bas (le plus strict) à chaque niveau. Si aucune règle
/// ne correspond aux flags déclenchés, on retombe sur les seuils par défaut.
pub fn resolve_thresholds(rules: &[Rule], fired: &[FlagType]) -> (f64, f64, f64, f64) {
    let relevant: Vec<&Rule> = rules
        .iter()
        .filter(|r| r.enabled && fired.contains(&r.flag_type))
        .collect();

    if relevant.is_empty() {
        return (
            DEFAULT_THRESHOLD_WARN,
            DEFAULT_THRESHOLD_DELETE,
            DEFAULT_THRESHOLD_MUTE,
            DEFAULT_THRESHOLD_BAN,
        );
    }

    let warn = relevant
        .iter()
        .map(|r| r.threshold_warn)
        .fold(f64::MAX, f64::min);
    let delete = relevant
        .iter()
        .map(|r| r.threshold_delete)
        .fold(f64::MAX, f64::min);
    let mute = relevant
        .iter()
        .map(|r| r.threshold_mute)
        .fold(f64::MAX, f64::min);
    let ban = relevant
        .iter()
        .map(|r| r.threshold_ban)
        .fold(f64::MAX, f64::min);

    (warn, delete, mute, ban)
}

#[cfg(test)]
#[path = "tests/scoring_service.rs"]
mod tests;

//! Types domaine du module Bump (recompense de bump multi-provider).

/// Rappel de bump du (guild, provider) dont le cooldown est ecoule et dont le
/// rappel n'a pas encore ete envoye (poll par le bot).
#[derive(Debug, Clone)]
pub struct DueReminder {
    pub guild_id: String,
    pub channel_id: String,
    pub provider: String,
}

/// Resultat d'un enregistrement de bump (recompense + info VIP).
#[derive(Debug, Clone)]
pub struct BumpReward {
    pub rewarded: bool,
    pub reward: i64,
    pub weekly_count: i64,
    pub new_balance: Option<i64>,
    /// Role VIP a attribuer (le bot fait l'ajout Discord, idempotent).
    pub vip_role_id: Option<String>,
    /// `true` uniquement au bump qui FRANCHIT le seuil (annonce unique).
    pub vip_just_unlocked: bool,
}

impl BumpReward {
    /// Resultat "pas de recompense" (module/provider off, ou cooldown non ecoule).
    pub fn none() -> Self {
        Self {
            rewarded: false,
            reward: 0,
            weekly_count: 0,
            new_balance: None,
            vip_role_id: None,
            vip_just_unlocked: false,
        }
    }
}

/// Recompense graduee : 1er bump = base ; chaque bump suppl. de la semaine ajoute
/// `step` ; plafonnee a `max`. `n` = Nieme bump de la semaine (>=1).
pub fn bump_reward(n: i64, base: i64, step: i64, max: i64) -> i64 {
    let raw = base + (n - 1).max(0) * step;
    raw.clamp(0, max.max(base))
}

/// Cle provider normalisee : alphanum minuscule uniquement (securise le
/// namespacing de config `{provider}_*` et la colonne `provider`).
pub fn sanitize_provider(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if cleaned.is_empty() {
        "disboard".to_string()
    } else {
        cleaned
    }
}

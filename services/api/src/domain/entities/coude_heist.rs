//! Systeme de braquage de la caisse communautaire (Phase 10).
//!
//! Une fois par semaine, un joueur peut tenter un gros coup sur la
//! caisse communautaire. Taux de base tres bas (5 %), boost par items
//! consommables (+5 % chacun, cap 50 %). Succes : gain aleatoire
//! 30-75 % de la caisse. Echec : prison 24 h, blocage total du jeu.
//!
//! **Choix d'architecture** : constantes hardcodees ici avec notes.
//! Modifier ici puis redeployer.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Cooldown entre deux tentatives de braquage (par joueur).
pub const HEIST_COOLDOWN_DAYS: i64 = 7;

/// Taux de succes de base sans aucun item.
pub const HEIST_BASE_SUCCESS_PERCENT: u32 = 5;

/// Bonus apporte par chaque item de braquage actif dans l'inventaire.
pub const HEIST_ITEM_BONUS_PERCENT: u32 = 5;

/// Plafond maximum du taux de succes (avec tous les items).
pub const HEIST_MAX_SUCCESS_PERCENT: u32 = 50;

/// Duree de la prison sur un echec.
pub const HEIST_PRISON_HOURS: i64 = 24;

/// Bornes du gain aleatoire en cas de succes (% de la caisse).
pub const HEIST_GAIN_MIN_PERCENT: u32 = 30;
pub const HEIST_GAIN_MAX_PERCENT: u32 = 75;

/// Definition d'un item consommable de braquage.
#[derive(Debug, Clone)]
pub struct HeistToolDef {
    pub key: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
    pub price: i64,
}

/// 9 items consommables. Chacun apporte +5 % de chance de succes quand
/// present dans l'inventaire au moment du braquage. 9 items × 5 % + 5 %
/// base = 50 % max (le cap). Tous les items actifs sont consommes quel
/// que soit le resultat du roll.
///
/// **Choix d'architecture** : catalog et prix hardcodes. Modifier ici
/// puis redeployer. Grille de prix en gradient pour que les meilleurs
/// items restent coherents avec l'economie.
pub const HEIST_TOOLS: &[HeistToolDef] = &[
    HeistToolDef {
        key: "masque_braquage",
        name: "Masque de braquage",
        emoji: "\u{1f3ad}",
        description: "+5 % de chance de reussite. Le classique.",
        price: 100,
    },
    HeistToolDef {
        key: "pied_de_biche",
        name: "Pied-de-biche",
        emoji: "\u{1f528}",
        description: "+5 % de chance. Pour forcer les portes arriere.",
        price: 150,
    },
    HeistToolDef {
        key: "crochet_vault",
        name: "Crochet de vault",
        emoji: "\u{1f513}",
        description: "+5 % de chance. Plus discret que l'explosif.",
        price: 220,
    },
    HeistToolDef {
        key: "plan_coffre",
        name: "Plan du coffre",
        emoji: "\u{1f5fa}\u{fe0f}",
        description: "+5 % de chance. La moitie du boulot est deja fait.",
        price: 320,
    },
    HeistToolDef {
        key: "fumigene_diversion",
        name: "Fumigene de diversion",
        emoji: "\u{1f4a8}",
        description: "+5 % de chance. Sors discret.",
        price: 450,
    },
    HeistToolDef {
        key: "explosif",
        name: "Explosif",
        emoji: "\u{1f4a3}",
        description: "+5 % de chance. La methode directe.",
        price: 600,
    },
    HeistToolDef {
        key: "hacker_kit",
        name: "Hacker kit",
        emoji: "\u{1f4be}",
        description: "+5 % de chance. Bypass total des alarmes.",
        price: 800,
    },
    HeistToolDef {
        key: "drone_espion",
        name: "Drone espion",
        emoji: "\u{1f681}",
        description: "+5 % de chance. Reperage aerien avant le coup.",
        price: 1000,
    },
    HeistToolDef {
        key: "equipe_de_pros",
        name: "Equipe de pros",
        emoji: "\u{1f46a}",
        description: "+5 % de chance. Tu n'es plus seul sur le coup.",
        price: 1500,
    },
];

pub fn find_heist_tool(key: &str) -> Option<&'static HeistToolDef> {
    HEIST_TOOLS.iter().find(|i| i.key == key)
}

/// Calcule le taux de succes effectif en fonction des items actifs.
/// `active_tool_keys` est la liste des item_keys de braquage presents
/// dans l'inventaire du joueur (doublons ignores).
pub fn compute_success_chance<I, S>(active_tool_keys: I) -> u32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    use std::collections::HashSet;
    let unique: HashSet<String> = active_tool_keys
        .into_iter()
        .filter(|k| find_heist_tool(k.as_ref()).is_some())
        .map(|k| k.as_ref().to_string())
        .collect();
    let bonus = (unique.len() as u32) * HEIST_ITEM_BONUS_PERCENT;
    (HEIST_BASE_SUCCESS_PERCENT + bonus).min(HEIST_MAX_SUCCESS_PERCENT)
}

/// Une tentative de braquage enregistree (log pour cooldown + historique).
#[derive(Debug, Clone)]
pub struct CoudeHeistAttempt {
    pub id: Uuid,
    pub guild_id: String,
    pub user_id: String,
    pub success: bool,
    pub amount_stolen: i64,
    pub chance_percent: i32,
    pub tools_used: Vec<String>,
    pub attempted_at: DateTime<Utc>,
}

/// Etat d'incarceration d'un joueur.
#[derive(Debug, Clone)]
pub struct CoudePrisonState {
    pub guild_id: String,
    pub user_id: String,
    pub released_at: DateTime<Utc>,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

impl CoudePrisonState {
    /// `true` si le joueur est actuellement en prison (released_at > now).
    pub fn is_active(&self) -> bool {
        self.released_at > Utc::now()
    }
}

/// Resultat d'une tentative de braquage, cuisine pour affichage par le bot.
#[derive(Debug, Clone)]
pub struct HeistOutcome {
    pub success: bool,
    pub chance_percent: u32,
    pub cashbox_total_before: i64,
    pub amount_stolen: i64,
    pub tools_consumed: Vec<String>,
    /// Si success == false : la date de liberation de prison.
    pub prison_released_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_chance_no_items_is_base() {
        let empty: Vec<&str> = vec![];
        assert_eq!(compute_success_chance(empty), HEIST_BASE_SUCCESS_PERCENT);
    }

    #[test]
    fn compute_chance_adds_bonus_per_unique_item() {
        let v = vec!["masque_braquage", "pied_de_biche"];
        // 5 + 2*5 = 15
        assert_eq!(compute_success_chance(v), 15);
    }

    #[test]
    fn compute_chance_ignores_unknown_items() {
        let v = vec!["masque_braquage", "unknown_tool"];
        assert_eq!(compute_success_chance(v), 10);
    }

    #[test]
    fn compute_chance_deduplicates_items() {
        // Si le joueur a 2x le meme item (bug futur ou migration), on
        // compte comme 1. Le meme item ne boost pas plusieurs fois.
        let v = vec!["masque_braquage", "masque_braquage"];
        assert_eq!(compute_success_chance(v), 10);
    }

    #[test]
    fn compute_chance_caps_at_max() {
        // Tous les 9 items : 5 + 9*5 = 50 (cap atteint pile).
        let v: Vec<&str> = HEIST_TOOLS.iter().map(|t| t.key).collect();
        assert_eq!(compute_success_chance(v), HEIST_MAX_SUCCESS_PERCENT);
    }

    #[test]
    fn catalog_has_exactly_9_tools() {
        assert_eq!(HEIST_TOOLS.len(), 9);
    }

    #[test]
    fn catalog_prices_are_ascending() {
        for pair in HEIST_TOOLS.windows(2) {
            assert!(
                pair[0].price <= pair[1].price,
                "catalog heist tools non tries par prix ascending : {} ({}) vs {} ({})",
                pair[0].key, pair[0].price, pair[1].key, pair[1].price
            );
        }
    }

    #[test]
    fn find_heist_tool_works() {
        assert!(find_heist_tool("masque_braquage").is_some());
        assert!(find_heist_tool("equipe_de_pros").is_some());
        assert!(find_heist_tool("unknown").is_none());
    }

    #[test]
    fn gain_range_is_sensible() {
        assert!(HEIST_GAIN_MIN_PERCENT < HEIST_GAIN_MAX_PERCENT);
        assert!(HEIST_GAIN_MAX_PERCENT <= 100);
    }
}

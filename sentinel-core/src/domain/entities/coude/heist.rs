//! Systeme de braquage de la caisse communautaire (Phase 10).
//!
//! Une fois par semaine, un joueur peut tenter un gros coup sur la
//! caisse communautaire. Taux de base tres bas (5 %), boost par items
//! consommables (+5 % chacun, cap 55 %). Succes : gain aleatoire
//! 30-75 % de la caisse. Echec : prison 24 h, blocage total du jeu.
//!
//! **Choix d'architecture** : constantes hardcodees ici avec notes.
//! Modifier ici puis redeployer.

use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;
use crate::domain::entities::system::discord_ids::UserId;
use crate::domain::entities::system::discord_ids::GuildId;

/// Cooldown entre deux tentatives de braquage (par joueur).
pub const HEIST_COOLDOWN_DAYS: i64 = 7;

/// Taux de succes de base sans aucun item.
pub const HEIST_BASE_SUCCESS_PERCENT: u32 = 5;

/// Ancien bonus fixe par item — remplace par `HeistToolDef.bonus_percent`
/// individuel. Conserve comme fallback si un item n'a pas de bonus defini.
pub const HEIST_ITEM_BONUS_PERCENT: u32 = 5;

/// Plafond maximum du taux de succes (avec tous les items).
/// Aligne sur `HEIST_BASE_SUCCESS_PERCENT (5) + sum(HEIST_TOOLS.bonus_percent) (50) = 55`
/// pour que le joueur qui possede tous les outils touche son bonus complet.
/// Avant : 50, ce qui jetait silencieusement 5 points de bonus (voir test
/// `catalog_bonus_sum_equals_max` pour le garde-fou).
pub const HEIST_MAX_SUCCESS_PERCENT: u32 = 55;

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
    /// Bonus de chance individuel (%). Les items chers boostent plus.
    pub bonus_percent: u32,
}

/// 9 items consommables. Chacun apporte un bonus de chance proportionnel
/// a son prix (2 % a 10 %). Somme des 9 bonus = 50 % (avec base 5 % →
/// cap 55 % atteint pile avec tous les outils). Tous les items actifs
/// sont consommes quel que soit le resultat du roll.
///
/// **Choix d'architecture** : catalog et prix hardcodes. Modifier ici
/// puis redeployer. Grille de prix en gradient pour que les meilleurs
/// items restent coherents avec l'economie.
pub const HEIST_TOOLS: &[HeistToolDef] = &[
    HeistToolDef {
        key: "masque_braquage",
        name: "Masque de braquage",
        emoji: "\u{1f3ad}",
        description: "+2 % de chance de reussite. Le classique.",
        price: 100,
        bonus_percent: 2,
    },
    HeistToolDef {
        key: "pied_de_biche",
        name: "Pied-de-biche",
        emoji: "\u{1f528}",
        description: "+3 % de chance. Pour forcer les portes arriere.",
        price: 150,
        bonus_percent: 3,
    },
    HeistToolDef {
        key: "crochet_vault",
        name: "Crochet de vault",
        emoji: "\u{1f513}",
        description: "+4 % de chance. Plus discret que l'explosif.",
        price: 220,
        bonus_percent: 4,
    },
    HeistToolDef {
        key: "plan_coffre",
        name: "Plan du coffre",
        emoji: "\u{1f5fa}\u{fe0f}",
        description: "+5 % de chance. La moitie du boulot est deja fait.",
        price: 320,
        bonus_percent: 5,
    },
    HeistToolDef {
        key: "fumigene_diversion",
        name: "Fumigene de diversion",
        emoji: "\u{1f4a8}",
        description: "+5 % de chance. Sors discret.",
        price: 450,
        bonus_percent: 5,
    },
    HeistToolDef {
        key: "explosif",
        name: "Explosif",
        emoji: "\u{1f4a3}",
        description: "+6 % de chance. La methode directe.",
        price: 600,
        bonus_percent: 6,
    },
    HeistToolDef {
        key: "hacker_kit",
        name: "Hacker kit",
        emoji: "\u{1f4be}",
        description: "+7 % de chance. Bypass total des alarmes.",
        price: 800,
        bonus_percent: 7,
    },
    HeistToolDef {
        key: "drone_espion",
        name: "Drone espion",
        emoji: "\u{1f681}",
        description: "+8 % de chance. Reperage aerien avant le coup.",
        price: 1000,
        bonus_percent: 8,
    },
    HeistToolDef {
        key: "equipe_de_pros",
        name: "Equipe de pros",
        emoji: "\u{1f46a}",
        description: "+10 % de chance. Tu n'es plus seul sur le coup.",
        price: 1500,
        bonus_percent: 10,
    },
];

pub fn find_heist_tool(key: &str) -> Option<&'static HeistToolDef> {
    HEIST_TOOLS.iter().find(|i| i.key == key)
}

/// Calcule le taux de succes effectif en fonction des items actifs.
/// `active_tool_keys` est la liste des item_keys de braquage presents
/// dans l'inventaire du joueur (doublons ignores). Chaque item apporte
/// son propre `bonus_percent` proportionnel a son prix.
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
    let bonus: u32 = unique
        .iter()
        .filter_map(|k| find_heist_tool(k))
        .map(|t| t.bonus_percent)
        .sum();
    (HEIST_BASE_SUCCESS_PERCENT + bonus).min(HEIST_MAX_SUCCESS_PERCENT)
}

/// Une tentative de braquage enregistree (log pour cooldown + historique).
#[derive(Debug, Clone)]
pub struct HeistAttempt {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub success: bool,
    pub amount_stolen: i64,
    pub chance_percent: i32,
    pub tools_used: Vec<String>,
    pub attempted_at: DateTime<Utc>,
}

/// Etat d'incarceration d'un joueur.
#[derive(Debug, Clone)]
pub struct PrisonState {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub released_at: DateTime<Utc>,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

impl PrisonState {
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
#[path = "tests/heist.rs"]
mod tests;

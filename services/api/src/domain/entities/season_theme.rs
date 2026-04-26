//! Saisons thematiques (cf. COUPE_AMELIORATIONS section 6.3).
//!
//! Chaque saison de 90 jours peut avoir un *theme* annonce qui module
//! l ambiance et (a terme) la mecanique. Premiere passe declarative :
//! le catalogue + un champ de config `current_season_theme` que les
//! admins du serveur posent via /saison-theme. Le bot l affiche
//! aux joueurs et les multiplicateurs documentes seront cables
//! progressivement (chaos x2, DEF +20%, etc.).

/// Cle de configuration ou le theme courant est stocke (table
/// `bot_guild_config`, scope `coude-bot`).
pub const CURRENT_SEASON_THEME_CONFIG_KEY: &str = "current_season_theme";

/// Un theme : identite + libelle + emoji + multiplicateurs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeasonTheme {
    pub key: &'static str,
    pub label: &'static str,
    pub emoji: &'static str,
    pub tagline: &'static str,
    /// Multiplicateur de proba des chaos events (1.0 = neutre, 2.0 = x2).
    pub chaos_multiplier: f64,
    /// Bonus de DEF pour les Tanks (en %).
    pub tank_def_bonus_pct: f64,
    /// Multiplicateur des gains de vol (1.0 = neutre, 1.5 = x1.5).
    pub steal_gain_multiplier: f64,
    /// Multiplicateur efficacite des protections vol.
    pub steal_protection_efficiency: f64,
    /// Multiplicateur du cooldown braquage (1.0 = neutre, 0.5 = /2).
    pub braquage_cooldown_multiplier: f64,
}

pub const SEASON_THEMES: &[SeasonTheme] = &[
    SeasonTheme {
        key: "chaos",
        label: "Saison du Chaos",
        emoji: "🌀",
        tagline: "Les events chaos sont x2 cette saison. Que la confusion regne !",
        chaos_multiplier: 2.0,
        tank_def_bonus_pct: 0.0,
        steal_gain_multiplier: 1.0,
        steal_protection_efficiency: 1.0,
        braquage_cooldown_multiplier: 1.0,
    },
    SeasonTheme {
        key: "tank",
        label: "Saison du Tank",
        emoji: "🪨",
        tagline: "+20% DEF pour les Tanks. Les Bourrins en bavent.",
        chaos_multiplier: 1.0,
        tank_def_bonus_pct: 20.0,
        steal_gain_multiplier: 1.0,
        steal_protection_efficiency: 1.0,
        braquage_cooldown_multiplier: 1.0,
    },
    SeasonTheme {
        key: "vol",
        label: "Saison du Vol",
        emoji: "🥷",
        tagline: "Gains de vol x1.5, mais protections -25%. Plus rentable, plus risque.",
        chaos_multiplier: 1.0,
        tank_def_bonus_pct: 0.0,
        steal_gain_multiplier: 1.5,
        steal_protection_efficiency: 0.75,
        braquage_cooldown_multiplier: 1.0,
    },
    SeasonTheme {
        key: "braquage",
        label: "Saison du Braquage",
        emoji: "🏦",
        tagline: "Cooldown braquage divise par 2. Saison des grands coups.",
        chaos_multiplier: 1.0,
        tank_def_bonus_pct: 0.0,
        steal_gain_multiplier: 1.0,
        steal_protection_efficiency: 1.0,
        braquage_cooldown_multiplier: 0.5,
    },
];

/// Lookup d un theme par sa cle. None si inconnue.
pub fn season_theme_by_key(key: &str) -> Option<&'static SeasonTheme> {
    SEASON_THEMES.iter().find(|t| t.key == key)
}

/// Selectionne automatiquement un theme en fonction du numero de saison
/// (rotation circulaire sur les 4 themes). Saisons 1,5,9... = Chaos,
/// 2,6,10... = Tank, 3,7,11... = Vol, 4,8,12... = Braquage. Saison 0 ou
/// negative = Chaos par defaut.
pub fn theme_for_season(season_number: i32) -> &'static SeasonTheme {
    if season_number <= 0 {
        return &SEASON_THEMES[0];
    }
    let idx = ((season_number - 1) as usize) % SEASON_THEMES.len();
    &SEASON_THEMES[idx]
}

#[cfg(test)]
#[path = "tests/season_theme.rs"]
mod tests;

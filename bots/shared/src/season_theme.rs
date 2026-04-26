//! Saisons thematiques (cf. COUPE_AMELIORATIONS section 6.3).
//!
//! Mirror cote bot du catalogue de themes defini dans
//! `services/api/src/domain/entities/season_theme.rs`. Synchronise
//! manuellement — tout changement doit etre repercute des deux cotes.
//!
//! La rotation est purement deterministe a partir du numero de saison
//! deja stocke dans `coude_players.season`.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeasonTheme {
    pub key: &'static str,
    pub label: &'static str,
    pub emoji: &'static str,
    pub tagline: &'static str,
}

pub const SEASON_THEMES: &[SeasonTheme] = &[
    SeasonTheme {
        key: "chaos",
        label: "Saison du Chaos",
        emoji: "🌀",
        tagline: "Les events chaos sont x2 cette saison. Que la confusion regne !",
    },
    SeasonTheme {
        key: "tank",
        label: "Saison du Tank",
        emoji: "🪨",
        tagline: "+20% DEF pour les Tanks. Les Bourrins en bavent.",
    },
    SeasonTheme {
        key: "vol",
        label: "Saison du Vol",
        emoji: "🥷",
        tagline: "Gains de vol x1.5, mais protections -25%. Plus rentable, plus risque.",
    },
    SeasonTheme {
        key: "braquage",
        label: "Saison du Braquage",
        emoji: "🏦",
        tagline: "Cooldown braquage divise par 2. Saison des grands coups.",
    },
];

/// Retourne le theme de la saison `season_number`. Rotation circulaire
/// sur les 4 themes (1=chaos, 2=tank, 3=vol, 4=braquage, 5=chaos, ...).
/// Saisons <= 0 -> Chaos par defaut.
pub fn theme_for_season(season_number: i32) -> &'static SeasonTheme {
    if season_number <= 0 {
        return &SEASON_THEMES[0];
    }
    let idx = ((season_number - 1) as usize) % SEASON_THEMES.len();
    &SEASON_THEMES[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_circular() {
        assert_eq!(theme_for_season(1).key, "chaos");
        assert_eq!(theme_for_season(2).key, "tank");
        assert_eq!(theme_for_season(3).key, "vol");
        assert_eq!(theme_for_season(4).key, "braquage");
        assert_eq!(theme_for_season(5).key, "chaos");
    }

    #[test]
    fn defaults_to_chaos_for_zero_or_negative() {
        assert_eq!(theme_for_season(0).key, "chaos");
        assert_eq!(theme_for_season(-5).key, "chaos");
    }

    #[test]
    fn four_themes() {
        assert_eq!(SEASON_THEMES.len(), 4);
    }
}

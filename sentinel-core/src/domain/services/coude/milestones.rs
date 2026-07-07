//! Paliers de niveau Coup de Coude (cf. COUPE_AMELIORATIONS 3.2).
//!
//! Regle metier PURE : la table des paliers (niveau -> deblocage) et la
//! regle du cooldown /repos effectif (palier "Convalescence" niveau 15 ->
//! plafond 8h) vivent ici, cote serveur, et non plus dans le bot.

/// Descripteur d'un palier de niveau (donnees d'affichage + seuil).
#[derive(Debug, Clone, Copy)]
pub struct Milestone {
    pub level: i32,
    pub key: &'static str,
    pub label: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
}

/// Niveau a partir duquel le cooldown /repos est reduit a
/// `REPOS_COOLDOWN_REDUCED_HOURS` heures (palier niveau 15).
pub const REPOS_COOLDOWN_MILESTONE_LEVEL: i32 = 15;
pub const REPOS_COOLDOWN_REDUCED_HOURS: i64 = 8;

/// Cooldown effectif de /repos pour un joueur de niveau `level` etant donne
/// le cooldown configure par le serveur (`base_hours`). Plafonne a
/// `REPOS_COOLDOWN_REDUCED_HOURS` si le joueur a debloque le palier
/// "Convalescence" (niveau 15+).
pub fn effective_repos_cooldown_hours(base_hours: i64, level: i32) -> i64 {
    if level >= REPOS_COOLDOWN_MILESTONE_LEVEL && base_hours > REPOS_COOLDOWN_REDUCED_HOURS {
        REPOS_COOLDOWN_REDUCED_HOURS
    } else {
        base_hours
    }
}

pub const MILESTONES: &[Milestone] = &[
    Milestone {
        level: 5,
        key: "extra_insurance_slot",
        label: "Coffre renforce",
        emoji: "\u{1f6e1}\u{fe0f}",
        description: "+1 emplacement d assurance (cumul de 2 actives au lieu de 1).",
    },
    Milestone {
        level: 10,
        key: "class_ultimate",
        label: "Ultime de classe",
        emoji: "\u{2b50}",
        description: "Debloque l ultime specifique a ta classe (cf. roadmap 3.1).",
    },
    Milestone {
        level: 15,
        key: "repos_short_cooldown",
        label: "Convalescence",
        emoji: "\u{1f6cf}\u{fe0f}",
        description: "Cooldown /repos reduit a 8h (au lieu de 12h).",
    },
    Milestone {
        level: 20,
        key: "riposte_first",
        label: "Riposte fulgurante",
        emoji: "\u{26a1}",
        description: "Priorite de riposte au round 1 quand tu te fais attaquer par un joueur de niveau inferieur.",
    },
    Milestone {
        level: 25,
        key: "prestige_unlock",
        label: "Acces au Prestige",
        emoji: "\u{1f451}",
        description: "Tu peux activer le Prestige (cf. roadmap 3.3) pour te relancer.",
    },
];

/// `true` si le palier est debloque par un joueur de niveau `level`.
pub fn is_unlocked(m: &Milestone, level: i32) -> bool {
    level >= m.level
}

/// Prochain palier a viser (level strictement > `level`). None si tout est
/// deja debloque.
pub fn next_for(level: i32) -> Option<&'static Milestone> {
    MILESTONES.iter().find(|m| m.level > level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_milestones_at_5_10_15_20_25() {
        let levels: Vec<i32> = MILESTONES.iter().map(|m| m.level).collect();
        assert_eq!(levels, vec![5, 10, 15, 20, 25]);
    }

    #[test]
    fn cooldown_low_level_returns_base() {
        assert_eq!(effective_repos_cooldown_hours(12, 1), 12);
        assert_eq!(effective_repos_cooldown_hours(12, 14), 12);
    }

    #[test]
    fn cooldown_at_milestone_caps_to_8() {
        assert_eq!(effective_repos_cooldown_hours(12, 15), 8);
        assert_eq!(effective_repos_cooldown_hours(12, 25), 8);
        assert_eq!(effective_repos_cooldown_hours(24, 15), 8);
    }

    #[test]
    fn cooldown_already_short_unchanged() {
        assert_eq!(effective_repos_cooldown_hours(6, 15), 6);
        assert_eq!(effective_repos_cooldown_hours(8, 15), 8);
    }

    #[test]
    fn next_for_boundaries() {
        assert_eq!(next_for(1).unwrap().level, 5);
        assert_eq!(next_for(5).unwrap().level, 10);
        assert!(next_for(25).is_none());
    }
}

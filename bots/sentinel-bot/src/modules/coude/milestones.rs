//! Paliers visibles (cf. COUPE_AMELIORATIONS section 3.2).
//!
//! 5 milestones permanents debloques aux niveaux 5/10/15/20/25. Cette
//! premiere passe est purement declarative : on affiche le palier
//! debloque + le prochain dans /profil. Les effets mecaniques (slot
//! assurance, ultimate, repos /3, riposte priority, prestige) seront
//! cables commit par commit cote services.

#[derive(Debug, Clone, Copy)]
pub struct Milestone {
    pub level: i32,
    pub key: &'static str,
    pub label: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
    /// `true` si l effet mecanique est branche, `false` si purement
    /// declaratif pour l instant.
    pub mechanical_implemented: bool,
}

/// Niveau a partir duquel le cooldown /repos est reduit a
/// `REPOS_COOLDOWN_REDUCED_HOURS` heures (cf. milestone niveau 15).
pub const REPOS_COOLDOWN_MILESTONE_LEVEL: i32 = 15;
pub const REPOS_COOLDOWN_REDUCED_HOURS: i64 = 8;

/// Retourne le cooldown effectif de /repos pour un joueur de niveau
/// `level` etant donne le cooldown configure par le serveur (`base_hours`).
/// Floor a `REPOS_COOLDOWN_REDUCED_HOURS` si le joueur a debloque le
/// palier "Convalescence" (niveau 15+).
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
        mechanical_implemented: false,
    },
    Milestone {
        level: 10,
        key: "class_ultimate",
        label: "Ultime de classe",
        emoji: "\u{2b50}",
        description: "Debloque l ultime specifique a ta classe (cf. roadmap 3.1).",
        mechanical_implemented: false,
    },
    Milestone {
        level: 15,
        key: "repos_short_cooldown",
        label: "Convalescence",
        emoji: "\u{1f6cf}\u{fe0f}",
        description: "Cooldown /repos reduit a 8h (au lieu de 12h).",
        mechanical_implemented: true,
    },
    Milestone {
        level: 20,
        key: "riposte_first",
        label: "Riposte fulgurante",
        emoji: "\u{26a1}",
        description: "Priorite de riposte au round 1 quand tu te fais attaquer par un joueur de niveau inferieur.",
        mechanical_implemented: true,
    },
    Milestone {
        level: 25,
        key: "prestige_unlock",
        label: "Acces au Prestige",
        emoji: "\u{1f451}",
        description: "Tu peux activer le Prestige (cf. roadmap 3.3) pour te relancer.",
        mechanical_implemented: false,
    },
];

/// Liste des paliers debloques par un joueur de niveau `level`.
pub fn unlocked_for(level: i32) -> Vec<&'static Milestone> {
    MILESTONES.iter().filter(|m| level >= m.level).collect()
}

/// Prochain palier a viser (level strictement > `level`). None si tout
/// est deja debloque.
pub fn next_for(level: i32) -> Option<&'static Milestone> {
    MILESTONES.iter().find(|m| m.level > level)
}

/// Resume compact pour /profil : "🛡️ Coffre renforce · ⭐ Ultime de classe"
/// + sous-ligne sur le prochain. Vide si pas debloque ET pas de prochain
/// (level >= 25).
pub fn format_profile_section(level: i32) -> String {
    let unlocked = unlocked_for(level);
    let next = next_for(level);
    let unlocked_line = if unlocked.is_empty() {
        "_Aucun palier debloque pour l instant._".to_string()
    } else {
        unlocked
            .iter()
            .map(|m| format!("{} **{}**", m.emoji, m.label))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    match next {
        Some(m) => format!(
            "{}\n\n\u{1f3af} Prochain : niveau **{}** -> {} {} ({})",
            unlocked_line, m.level, m.emoji, m.label, m.description
        ),
        None => format!("{}\n\n\u{1f3c6} Tous les paliers debloques !", unlocked_line),
    }
}

#[cfg(test)]
mod tests;

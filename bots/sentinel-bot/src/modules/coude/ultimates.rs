//! Ultimates par classe (cf. COUPE_AMELIORATIONS section 3.1).
//!
//! 4 ultimates thematiques debloques au niveau 10, 1 utilisation par
//! semaine via /ultimate. Cette premiere passe est purement declarative :
//! catalog + commande de consultation. Les effets mecaniques (HP swap,
//! coin flip, vol pre-combat, statue) seront cables commit par commit.

#[derive(Debug, Clone, Copy)]
pub struct ClassUltimate {
    pub class_key: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// `true` si l effet mecanique est branche, `false` si purement
    /// declaratif pour l instant.
    pub mechanical_implemented: bool,
    /// Cooldown en jours entre 2 utilisations.
    pub cooldown_days: i64,
}

/// Niveau a partir duquel l ultimate de classe est debloquee.
pub const ULTIMATE_UNLOCK_LEVEL: i32 = 10;

pub const CLASS_ULTIMATES: &[ClassUltimate] = &[
    ClassUltimate {
        class_key: "bourrin",
        name: "Bourrin",
        emoji: "\u{1f504}",
        label: "Echange de carcasses",
        description: "Swap ton HP courant avec celui de l adversaire AVANT le combat. Mourant a 5 HP ? Il herite de tes 5 HP et tu recuperes ses 180.",
        mechanical_implemented: true,
        cooldown_days: 7,
    },
    ClassUltimate {
        class_key: "agile",
        name: "Agile",
        emoji: "\u{1fa99}",
        label: "Pile ou face",
        description: "Combat instantanement resolu sur un 50/50 pur. Ignore classes, niveaux, items, HP, tout. Juste un coin flip.",
        mechanical_implemented: false,
        cooldown_days: 7,
    },
    ClassUltimate {
        class_key: "fourbe",
        name: "Fourbe",
        emoji: "\u{1f3c3}",
        label: "Le Fuyard",
        description: "Vol la mise AVANT le combat et te barre. Le defenseur recoit « ton adversaire a fui avec la caisse ». Cooldown 14 jours.",
        mechanical_implemented: false,
        cooldown_days: 14,
    },
    ClassUltimate {
        class_key: "tank",
        name: "Tank",
        emoji: "\u{1f9f1}",
        label: "Statue",
        description: "Aucun degat fait, aucun degat pris. Victoire automatique au bout de 10 rounds par forfait d ennui de l adversaire.",
        mechanical_implemented: false,
        cooldown_days: 7,
    },
];

/// Lookup par class_key.
pub fn ultimate_for_class(class_key: &str) -> Option<&'static ClassUltimate> {
    CLASS_ULTIMATES.iter().find(|u| u.class_key == class_key)
}

/// Resume compact pour /aide ou /profil :
/// "🔄 Echange de carcasses (debloque) / cooldown N jours".
pub fn format_ultimate_for_class(class_key: &str, level: i32) -> String {
    let Some(u) = ultimate_for_class(class_key) else {
        return "_(pas d ultimate pour cette classe)_".to_string();
    };
    if level < ULTIMATE_UNLOCK_LEVEL {
        format!(
            "{} **{}** — _Verrouille_ (debloque au niveau {})",
            u.emoji, u.label, ULTIMATE_UNLOCK_LEVEL
        )
    } else {
        format!(
            "{} **{}** — {} _(cooldown {} jours{})_",
            u.emoji,
            u.label,
            u.description,
            u.cooldown_days,
            if u.mechanical_implemented {
                ""
            } else {
                ", effet a venir"
            }
        )
    }
}

#[cfg(test)]
mod tests;

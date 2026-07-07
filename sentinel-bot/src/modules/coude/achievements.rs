//! Achievements cosmetiques (cf. COUPE_AMELIORATIONS section 3.4).
//!
//! 30+ succes purement derives de l etat actuel du joueur — aucune
//! persistance dedie n est necessaire, on recalcule a la volee depuis
//! les compteurs deja stockes dans `coude_players`.
//!
//! Aucun avantage gameplay, juste des badges affiches dans `/profil`.

/// Un succes : identite stable + libelle + emoji + critere de deblocage.
///
/// `label` et `description` ne sont pas consommes par le runtime du bot
/// (les embeds construisent leur texte depuis des templates dedies) mais
/// restent exposes pour l'introspection (dashboard / `/aide` futur).
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Achievement {
    pub key: &'static str,
    pub label: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
}

/// Les 30 succes disponibles. Ordre = ordre d affichage suggere
/// (combats -> economie -> casino -> niveau -> classe).
pub const ACHIEVEMENTS: &[Achievement] = &[
    // ── Combat ──
    Achievement {
        key: "first_blood",
        label: "Premier sang",
        emoji: "🩸",
        description: "Remporte ton premier combat",
    },
    Achievement {
        key: "veteran",
        label: "Veteran",
        emoji: "🎖️",
        description: "10 victoires",
    },
    Achievement {
        key: "butcher",
        label: "Boucher",
        emoji: "🪓",
        description: "50 victoires",
    },
    Achievement {
        key: "legend",
        label: "Legende",
        emoji: "🏆",
        description: "100 victoires",
    },
    Achievement {
        key: "punching_ball",
        label: "Punching ball",
        emoji: "🥊",
        description: "10 defaites",
    },
    Achievement {
        key: "tapis",
        label: "Tapis",
        emoji: "🛏️",
        description: "50 defaites",
    },
    Achievement {
        key: "diplomat",
        label: "Diplomate",
        emoji: "🤝",
        description: "20 matchs nuls",
    },
    Achievement {
        key: "no_quarter",
        label: "Pas de quartier",
        emoji: "💀",
        description: "20 victoires sans match nul",
    },
    // ── Lachete / Chaos ──
    Achievement {
        key: "coward_obvious",
        label: "Lache officiel",
        emoji: "🐔",
        description: "5 refus de combat",
    },
    Achievement {
        key: "coward_notorious",
        label: "Lache notoire",
        emoji: "🪶",
        description: "20 refus de combat",
    },
    Achievement {
        key: "chaos_king",
        label: "Roi du chaos",
        emoji: "🌀",
        description: "10 events chaos declenches",
    },
    Achievement {
        key: "chaos_master",
        label: "Maitre du chaos",
        emoji: "🌪️",
        description: "50 events chaos declenches",
    },
    // ── Vol ──
    Achievement {
        key: "first_heist",
        label: "Premier hold-up",
        emoji: "🦹",
        description: "1c vole",
    },
    Achievement {
        key: "pickpocket",
        label: "Pickpocket",
        emoji: "🪙",
        description: "1 000c voles cumules",
    },
    Achievement {
        key: "pro_thief",
        label: "Voleur pro",
        emoji: "🥷",
        description: "10 000c voles cumules",
    },
    Achievement {
        key: "bank_robber",
        label: "Cambrioleur de banque",
        emoji: "🏦",
        description: "100 000c voles cumules",
    },
    // ── Economie ──
    Achievement {
        key: "rich",
        label: "Riche",
        emoji: "💰",
        description: "10 000c en poche",
    },
    Achievement {
        key: "millionaire",
        label: "Millionnaire",
        emoji: "💎",
        description: "100 000c en poche",
    },
    Achievement {
        key: "magnate",
        label: "Magnat",
        emoji: "👑",
        description: "1 000 000c en poche",
    },
    Achievement {
        key: "investor",
        label: "Investisseur",
        emoji: "📈",
        description: "200 000c gagnes cumules",
    },
    Achievement {
        key: "bankrupt",
        label: "Faillite",
        emoji: "📉",
        description: "100 000c perdus cumules",
    },
    // ── Casino ──
    Achievement {
        key: "casino_addict",
        label: "Casino addict",
        emoji: "🎲",
        description: "10 actions casino",
    },
    Achievement {
        key: "lucky",
        label: "Chanceux",
        emoji: "🍀",
        description: "20 victoires casino",
    },
    Achievement {
        key: "casino_cursed",
        label: "Maudit du casino",
        emoji: "🎰",
        description: "20 defaites casino",
    },
    // ── Niveau ──
    Achievement {
        key: "apprentice",
        label: "Apprenti",
        emoji: "🎓",
        description: "Niveau 5",
    },
    Achievement {
        key: "veteran_play",
        label: "Veteran du jeu",
        emoji: "⚔️",
        description: "Niveau 10",
    },
    Achievement {
        key: "guardian",
        label: "Gardien",
        emoji: "🛡️",
        description: "Niveau 15",
    },
    Achievement {
        key: "ascetic",
        label: "Ascete",
        emoji: "🧘",
        description: "Niveau 20",
    },
    Achievement {
        key: "master",
        label: "Maitre",
        emoji: "🌟",
        description: "Niveau 25",
    },
    // ── Stats ──
    Achievement {
        key: "tank",
        label: "Tank",
        emoji: "🪨",
        description: "DEF >= 50 (statique)",
    },
    Achievement {
        key: "brute",
        label: "Brute",
        emoji: "💪",
        description: "ATK >= 50 (statique)",
    },
    Achievement {
        key: "specialist",
        label: "Specialiste",
        emoji: "🎭",
        description: "Une classe choisie",
    },
];

/// Emoji d'affichage pour une clef de succes (None si clef inconnue).
///
/// Le bareme (quels succes sont debloques) est resolu server-side ; le bot ne
/// conserve ce catalogue que pour mapper les clefs recues vers leurs emojis.
pub fn emoji_for_key(key: &str) -> Option<&'static str> {
    ACHIEVEMENTS
        .iter()
        .find(|a| a.key == key)
        .map(|a| a.emoji)
}

/// Resume compact pour /profil : "🩸 🎖️ 💰 ..." ou "_Aucun_" si vide.
///
/// La liste des clefs debloquees vient de l'API (`PlayerProgression`) ; le bot
/// ne fait que la rendre.
pub fn format_unlocked_compact(
    progression: &crate::modules::coude::api_client::PlayerProgression,
) -> String {
    if progression.unlocked_achievements.is_empty() {
        "_Aucun encore — joue pour en debloquer !_".into()
    } else {
        let line = progression
            .unlocked_achievements
            .iter()
            .filter_map(|k| emoji_for_key(k))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "{}\n_{} / {} succes_",
            line,
            progression.unlocked_achievements.len(),
            progression.total_achievements,
        )
    }
}

#[cfg(test)]
mod tests;

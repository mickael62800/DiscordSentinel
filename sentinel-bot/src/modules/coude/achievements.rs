//! Achievements cosmetiques (cf. COUPE_AMELIORATIONS section 3.4).
//!
//! 30+ succes purement derives de l etat actuel du joueur — aucune
//! persistance dedie n est necessaire, on recalcule a la volee depuis
//! les compteurs deja stockes dans `coude_players`.
//!
//! Aucun avantage gameplay, juste des badges affiches dans `/profil`.

use crate::modules::coude::api_client::Player;

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

/// Retourne `true` si le joueur a debloque ce succes.
pub fn is_unlocked(ach: &Achievement, p: &Player) -> bool {
    match ach.key {
        // Combat
        "first_blood" => p.total_wins >= 1,
        "veteran" => p.total_wins >= 10,
        "butcher" => p.total_wins >= 50,
        "legend" => p.total_wins >= 100,
        "punching_ball" => p.total_losses >= 10,
        "tapis" => p.total_losses >= 50,
        "diplomat" => p.total_draws >= 20,
        "no_quarter" => p.total_wins >= 20 && p.total_draws == 0,
        // Lachete / Chaos
        "coward_obvious" => p.cowardice_count >= 5,
        "coward_notorious" => p.cowardice_count >= 20,
        "chaos_king" => p.chaos_events >= 10,
        "chaos_master" => p.chaos_events >= 50,
        // Vol
        "first_heist" => p.total_stolen >= 1,
        "pickpocket" => p.total_stolen >= 1_000,
        "pro_thief" => p.total_stolen >= 10_000,
        "bank_robber" => p.total_stolen >= 100_000,
        // Economie
        "rich" => p.coins >= 10_000,
        "millionaire" => p.coins >= 100_000,
        "magnate" => p.coins >= 1_000_000,
        "investor" => p.total_earned >= 200_000,
        "bankrupt" => p.total_lost >= 100_000,
        // Casino
        "casino_addict" => (p.casino_wins + p.casino_losses) >= 10,
        "lucky" => p.casino_wins >= 20,
        "casino_cursed" => p.casino_losses >= 20,
        // Niveau
        "apprentice" => p.level >= 5,
        "veteran_play" => p.level >= 10,
        "guardian" => p.level >= 15,
        "ascetic" => p.level >= 20,
        "master" => p.level >= 25,
        // Stats
        "tank" => p.def >= 50,
        "brute" => p.atk >= 50,
        "specialist" => matches!(&p.class, Some(c) if !c.is_empty()),
        _ => false,
    }
}

/// Retourne tous les succes debloques par le joueur (ordre du catalogue).
pub fn unlocked_for(player: &Player) -> Vec<Achievement> {
    ACHIEVEMENTS
        .iter()
        .filter(|a| is_unlocked(a, player))
        .copied()
        .collect()
}

/// Resume compact pour /profil : "🩸 🎖️ 💰 ..." ou "_Aucun_" si vide.
pub fn format_unlocked_compact(player: &Player) -> String {
    let unlocked = unlocked_for(player);
    if unlocked.is_empty() {
        "_Aucun encore — joue pour en debloquer !_".into()
    } else {
        let line = unlocked
            .iter()
            .map(|a| a.emoji)
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "{}\n_{} / {} succes_",
            line,
            unlocked.len(),
            ACHIEVEMENTS.len()
        )
    }
}

#[cfg(test)]
mod tests;

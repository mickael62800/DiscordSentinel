//! Evenements chaos Mythiques (cf. COUPE_AMELIORATIONS section 2.1).
//!
//! 10 events tres rares (somme des probas ~1.62%), tous absurdes, tous
//! annonces avec ping serveur quand ils tombent. Quand un de ces trucs
//! arrive, tout le serveur en parle pendant 3 jours — la rarete cree la
//! legende.
//!
//! Cette premiere passe est PUREMENT DECLARATIVE : catalogue + roller +
//! annonce embed. Les effets mecaniques (draw force, resurrection, swap
//! classes, vol, etc.) sont marques `mechanical_implemented: false` pour
//! l instant et seront branches commit par commit.

use rand::Rng;

/// Une mythique : identite + libelle + ce qu il se passe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MythicEvent {
    pub key: &'static str,
    pub label: &'static str,
    /// Probabilite (0..1) de tirage par combat.
    pub probability: f64,
    pub emoji: &'static str,
    /// Texte d annonce (placeholders {atk}/{def}/{winner}/{loser} substitues
    /// au moment du dispatch).
    pub announce: &'static str,
    /// `true` si l effet mecanique est branche dans le moteur, `false` si
    /// l event est annonce mais sans impact sur le combat.
    pub mechanical_implemented: bool,
}

/// Les 10 mythiques. Ordre = ordre du roll (priorite stable pour les tests).
pub const MYTHIC_EVENTS: &[MythicEvent] = &[
    MythicEvent {
        key: "licorne_rose",
        label: "Licorne rose",
        probability: 0.0005,
        emoji: "🦄",
        announce: "🦄 **LICORNE ROSE** 🦄 — match nul force, +500c bonus pour les deux !",
        mechanical_implemented: true,
    },
    MythicEvent {
        key: "etoile_filante",
        label: "Etoile filante",
        probability: 0.002,
        emoji: "🌠",
        announce: "🌠 **ETOILE FILANTE** — les deux combattants ressuscitent a 100%, sudden death !",
        mechanical_implemented: false,
    },
    MythicEvent {
        key: "jackpot_divin",
        label: "Jackpot divin",
        probability: 0.001,
        emoji: "🎰",
        announce: "🎰 **JACKPOT DIVIN** — {winner} touche x10 la mise sortie de la cagnotte serveur !",
        mechanical_implemented: false,
    },
    MythicEvent {
        key: "revanche_outre_tombe",
        label: "Revanche d outre-tombe",
        probability: 0.003,
        emoji: "💀",
        announce: "💀 **REVANCHE D OUTRE-TOMBE** — {loser} ressuscite, vole 30% des coins de {winner}, repart !",
        mechanical_implemented: false,
    },
    MythicEvent {
        key: "invasion_poulets",
        label: "Invasion de poulets",
        probability: 0.002,
        emoji: "🐔",
        announce: "🐔 **INVASION DE POULETS** — 50 poulets sauvages envahissent l arene, le combat est annule, match nul !",
        mechanical_implemented: true,
    },
    MythicEvent {
        key: "distributeur_pq",
        label: "Distributeur de PQ",
        probability: 0.0015,
        emoji: "🧻",
        announce: "🧻 **DISTRIBUTEUR DE PQ** — tout le pot devient du PQ, personne ne gagne rien, on rigole quand meme.",
        mechanical_implemented: true,
    },
    MythicEvent {
        key: "trefle_quatre_feuilles",
        label: "Trefle a 4 feuilles",
        probability: 0.005,
        emoji: "🍀",
        announce: "🍀 **TREFLE A 4 FEUILLES** — {loser} recupere 150% de sa mise au lieu d en perdre. {winner} reste loggue gagnant.",
        mechanical_implemented: false,
    },
    MythicEvent {
        key: "aliens",
        label: "Aliens",
        probability: 0.0005,
        emoji: "🛸",
        announce: "🛸 **ALIENS** — les deux combattants sont abductes ! Combat marque 'non resolu' pendant 24h, resultat mystere a venir.",
        mechanical_implemented: false,
    },
    MythicEvent {
        key: "magicien",
        label: "Le Magicien",
        probability: 0.001,
        emoji: "🎩",
        announce: "🎩 **LE MAGICIEN** — les classes des deux combattants sont echangees pour ce combat seulement !",
        mechanical_implemented: false,
    },
    MythicEvent {
        key: "bombe_nucleaire",
        label: "Bombe nucleaire",
        probability: 0.0002,
        emoji: "💣",
        announce: "💣 **BOMBE NUCLEAIRE** 💣 — annihilation totale, les deux perdent 50% de leur wallet. La legende racontera ce combat.",
        mechanical_implemented: true,
    },
];

/// Tire au plus un mythique pour ce combat. Iter le catalogue dans l ordre,
/// premier hit (proba_roll < proba_event) gagne. Comme la somme des proba
/// est ~1.6%, deux events concurrents quasi-impossibles ; on prend le 1er.
///
/// `proba_roll` doit etre un f64 dans [0, 1).
pub fn roll_mythic_event(rng: &mut impl Rng) -> Option<MythicEvent> {
    // On tire un proba unique [0, 1) et on cumule les seuils — equivalent
    // a un weighted draw avec un "no event" en complement.
    let p: f64 = rng.gen_range(0.0..1.0);
    let mut cumul = 0.0;
    for ev in MYTHIC_EVENTS {
        cumul += ev.probability;
        if p < cumul {
            return Some(*ev);
        }
    }
    None
}

/// Substitue les placeholders dans le texte d annonce.
pub fn format_mythic_announce(
    ev: &MythicEvent,
    attacker_name: &str,
    defender_name: &str,
    winner_name: Option<&str>,
    loser_name: Option<&str>,
) -> String {
    ev.announce
        .replace("{atk}", attacker_name)
        .replace("{def}", defender_name)
        .replace("{winner}", winner_name.unwrap_or("le gagnant"))
        .replace("{loser}", loser_name.unwrap_or("le perdant"))
}

#[cfg(test)]
#[path = "tests/mythic_events.rs"]
mod tests;

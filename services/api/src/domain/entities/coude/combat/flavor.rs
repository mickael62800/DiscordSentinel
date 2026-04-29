//! Commentaires de combat debiles (cf. COUPE_AMELIORATIONS section 2.2).
//!
//! Catalogue de phrases ridicules glissees aleatoirement entre les rounds
//! d un combat (~20% de chance par round). Aucun impact mecanique : pure
//! ambiance. Les `{atk}` / `{def}` sont substitues par les noms des
//! combattants.
//!
//! Logique pure — testable sans contexte serveur.

use rand::Rng;

/// Probabilite (0..1) qu une ligne soit injectee a la fin d un round.
pub const FLAVOR_LINE_PROBABILITY: f64 = 0.20;

/// Catalogue de templates. `{atk}` et `{def}` sont les seuls placeholders.
pub const FLAVOR_LINES: &[&str] = &[
    "{atk} trebuche sur une echarde emotionnelle",
    "{def} refuse de se battre tant qu il n a pas fini son cafe",
    "{atk} utilise l attaque speciale **Mon pere est avocat** — aucun effet",
    "{def} hurle une citation de Confucius mal traduite — tout le monde est confus",
    "{atk} sort une banane de sa poche, ca n aide pas",
    "L arbitre fantome siffle un hors-jeu inexistant",
    "Un pigeon traverse l arene et fait caca sur l epaule de {def}",
    "{atk} fait semblant de relacer ses chaussures pendant 4 secondes",
    "{def} cherche son inhalateur, le trouve dans la mauvaise poche",
    "Quelqu un dans le public crie « C est nul », personne ne sait qui",
    "{atk} declare forfait, change d avis, redeclare forfait, recommence",
    "{def} regarde son adversaire dans les yeux, perd 1 round mental",
    "Le commentateur officiel s endort en plein milieu de l action",
    "{atk} essaie un mouvement vu sur TikTok — ca ne marche pas",
    "{def} se mord la levre par concentration, mal de bouche garanti",
    "Un papillon se pose sur le nez de {atk}, l ambiance est rompue",
    "{def} demande un temps mort pour aller pisser — refuse",
    "{atk} murmure « j ai oublie d eteindre le four » — concentration -10",
    "Une mouche atterrit dans le bol de soupe imaginaire de {def}",
    "{atk} fait l avion avec ses bras pendant 2 secondes pour aucune raison",
];

/// Choisit aleatoirement une ligne du catalogue et substitue les noms.
/// Retourne None si la probabilite n est pas atteinte (rng tirage > seuil).
///
/// `proba_roll` doit etre dans [0, 1).
pub fn pick_flavor_line(
    rng: &mut impl Rng,
    proba_roll: f64,
    attacker_name: &str,
    defender_name: &str,
) -> Option<String> {
    if proba_roll >= FLAVOR_LINE_PROBABILITY {
        return None;
    }
    if FLAVOR_LINES.is_empty() {
        return None;
    }
    let idx = rng.gen_range(0..FLAVOR_LINES.len());
    Some(
        FLAVOR_LINES[idx]
            .replace("{atk}", attacker_name)
            .replace("{def}", defender_name),
    )
}

#[cfg(test)]
#[path = "tests/flavor.rs"]
mod tests;

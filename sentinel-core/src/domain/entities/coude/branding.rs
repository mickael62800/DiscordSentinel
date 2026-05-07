//! Identite / tagline du jeu (cf. COUPE_AMELIORATIONS section 6.4).
//!
//! Constante centrale utilisee dans les footers d embed pour renforcer
//! l identite "Coup de Coude — Le jeu Discord ou le chaos gagne toujours".

/// Tagline principale a afficher dans les footers d embed des modules
/// games (coude / blackjack / slot / wheel) et dans /aide.
pub const COUDE_TAGLINE: &str = "Coup de Coude — Le jeu ou le chaos gagne toujours.";

/// Tagline raccourcie pour les footers minimalistes.
pub const COUDE_TAGLINE_SHORT: &str = "Le chaos gagne toujours.";

/// Tagline globale du serveur Sentinel (utilisable dans /help / /info).
pub const SENTINEL_TAGLINE: &str = "Sentinel — Combats. Paris. Vols. Surtout : survis.";

/// Footer formate pour un embed de combat coude.
pub fn coude_combat_footer(round_count: usize) -> String {
    format!("{} | {} round{}", COUDE_TAGLINE_SHORT, round_count, if round_count > 1 { "s" } else { "" })
}

/// Footer formate pour un embed de paris.
pub fn coude_bet_footer(total_pot: i64) -> String {
    format!("{} | Pot total : {} c", COUDE_TAGLINE_SHORT, total_pot)
}

#[cfg(test)]
#[path = "tests/branding.rs"]
mod tests;

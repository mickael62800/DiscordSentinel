//! Systeme de Prestige (cf. COUPE_AMELIORATIONS 3.3).
//!
//! Au niveau 25, un joueur peut "Prestige" : reset au niveau 1 mais
//! gagne +5% de gains permanents par prestige (cumul). Cap a 5
//! prestiges (=+25% gains perma + 5 etoiles).

/// Niveau a atteindre pour pouvoir activer le Prestige.
pub const PRESTIGE_UNLOCK_LEVEL: i32 = 25;

/// Nombre maximum de prestiges par joueur (cap).
pub const PRESTIGE_MAX_COUNT: i32 = 5;

/// Bonus de gain par prestige (5%).
pub const PRESTIGE_GAIN_BONUS_PCT: f64 = 0.05;

/// Verifie si un joueur peut activer un nouveau prestige.
pub fn can_prestige(level: i32, current_prestige_count: i32) -> bool {
    level >= PRESTIGE_UNLOCK_LEVEL && current_prestige_count < PRESTIGE_MAX_COUNT
}

/// Multiplicateur de gain applique aux gains du joueur en fonction de
/// son nombre de prestiges. 1.0 = neutre, 1.25 = 5 prestiges max.
pub fn prestige_gain_multiplier(prestige_count: i32) -> f64 {
    let count = prestige_count.clamp(0, PRESTIGE_MAX_COUNT);
    1.0 + (count as f64 * PRESTIGE_GAIN_BONUS_PCT)
}

/// Format etoiles pour affichage (1 prestige = ⭐, 5 = ⭐⭐⭐⭐⭐).
pub fn prestige_stars(prestige_count: i32) -> String {
    let count = prestige_count.clamp(0, PRESTIGE_MAX_COUNT);
    "\u{2b50}".repeat(count as usize)
}

#[cfg(test)]
#[path = "tests/prestige.rs"]
mod tests;

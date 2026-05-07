/// Niveau maximum atteignable.
pub const MAX_LEVEL: i32 = 25;

/// XP cumul total requis pour atteindre le niveau `level`.
///
/// **Source unique de verite** pour la progression. Le bot recupere ces
/// valeurs via le RPC `GetCatalog` au boot (table pre-calculee). Ne calcule
/// jamais localement.
pub fn xp_for_level(level: i32) -> i64 {
    (50 * level * level + 50 * level) as i64
}

/// Titre correspondant au niveau.
pub fn title_for_level(level: i32) -> &'static str {
    match level {
        1..=4 => "Debutant",
        5..=9 => "Bagarreur",
        10..=14 => "Guerrier",
        15..=19 => "Veteran",
        20..=24 => "Champion",
        25 => "Inarretable",
        _ => "Debutant",
    }
}

/// Calcule le handicap de matchmaking.
/// Retourne (multiplicateur_atk_pour_le_plus_fort, est_bloque).
pub fn matchmaking_handicap(attacker_level: i32, defender_level: i32) -> (f64, bool) {
    let gap = (attacker_level - defender_level).abs();
    match gap {
        0..=2 => (1.0, false),
        3..=5 => (0.8, false),
        6..=9 => (0.6, false),
        _ => (0.0, true),
    }
}

#[cfg(test)]
#[path = "tests/progression.rs"]
mod tests;

//! Reduction de friction sociale (cf. COUPE_AMELIORATIONS section 4.2).
//!
//! Si le defenseur est trop bas en HP au moment ou il refuse un combat,
//! on n incremente PAS son `cowardice_count`. C est legitime de refuser
//! un combat quand on est mourant — pas de la lachete.

/// Seuil de HP (% du HP max) en dessous duquel un refus n est PAS compte
/// comme un acte de lachete. Recommandation : 20%.
pub const COWARDICE_RELIEF_HP_PCT: f64 = 0.20;

/// Doit-on incrementer le compteur cowardice du defenseur quand il refuse ?
///
/// Retourne `false` si le defenseur est suffisamment bas pour beneficier
/// du relief — son refus est legitime.
pub fn should_count_as_cowardice(defender_hp_remaining: i32, defender_hp_max: i32) -> bool {
    if defender_hp_max <= 0 {
        // Defensif : si pas d info HP fiable, on garde le comportement
        // historique (= ne pas exempter, le compteur s incrementera).
        return true;
    }
    let pct = (defender_hp_remaining as f64 / defender_hp_max as f64).clamp(0.0, 1.0);
    pct > COWARDICE_RELIEF_HP_PCT
}

#[cfg(test)]
#[path = "tests/cowardice_relief.rs"]
mod tests;

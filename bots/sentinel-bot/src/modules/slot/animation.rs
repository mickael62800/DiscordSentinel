//! Animation suspense du spin : revele les 3 symboles progressivement
//! sur 6 secondes (3 frames de 2s) en editant un message Discord.
//!
//! Logique pure (calcul des frames) testable sans Discord.

/// Embleme spinning quand un emplacement n est pas encore revele.
pub const SPINNING_PLACEHOLDER: &str = "\u{1f3b0}"; // 🎰

/// Construit l affichage des 3 symboles a la frame `frame_idx` (0..=3).
///
/// - frame 0 : aucun symbole revele -> `🎰 🎰 🎰`
/// - frame 1 : 1er symbole revele
/// - frame 2 : 1er + 2eme symboles reveles
/// - frame 3 (et au-dela) : tous reveles
///
/// `final_symbols` doit contenir exactement 3 elements.
pub fn frame_symbols(final_symbols: &[String; 3], frame_idx: usize) -> [String; 3] {
    let mut out = [
        SPINNING_PLACEHOLDER.to_string(),
        SPINNING_PLACEHOLDER.to_string(),
        SPINNING_PLACEHOLDER.to_string(),
    ];
    let revealed = frame_idx.min(3);
    #[allow(clippy::manual_memcpy)]
    for i in 0..revealed {
        out[i] = final_symbols[i].clone();
    }
    out
}

/// Nombre total de frames intermediaires (avant le frame final).
/// 3 frames revelantes + le frame final = 4 frames distinctes au total.
pub const TOTAL_REVEAL_FRAMES: usize = 3;

/// Delai entre 2 frames en millisecondes (2 secondes).
pub const FRAME_DELAY_MS: u64 = 2000;

#[cfg(test)]
#[path = "tests/animation.rs"]
mod tests;

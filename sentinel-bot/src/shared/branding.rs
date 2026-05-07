//! Identite / tagline partagee bot + API.
//!
//! Permet de garantir une coherence dans tous les embeds (footers
//! notamment). Synchronise avec `sentinel-api/src/domain/entities/branding.rs`.

/// Tagline principale Coup de Coude.
pub const COUDE_TAGLINE: &str = "Coup de Coude — Le jeu ou le chaos gagne toujours.";

/// Tagline raccourcie pour les footers minimalistes.
pub const COUDE_TAGLINE_SHORT: &str = "Coup de Coude · Le chaos gagne toujours.";

/// Tagline globale du serveur Sentinel.
pub const SENTINEL_TAGLINE: &str = "Sentinel — Combats. Paris. Vols. Surtout : survis.";

/// Tagline blackjack.
pub const BLACKJACK_TAGLINE: &str = "Blackjack · La maison gagne souvent. Mais pas tout le temps.";

/// Tagline slot machine.
pub const SLOT_TAGLINE: &str = "Slot Machine · Tire la chance par la queue.";

/// Tagline Roue du Destin.
pub const WHEEL_TAGLINE: &str = "Roue du Destin · Une chance par jour. Le destin decide.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_taglines_non_empty() {
        for t in [
            COUDE_TAGLINE, COUDE_TAGLINE_SHORT, SENTINEL_TAGLINE,
            BLACKJACK_TAGLINE, SLOT_TAGLINE, WHEEL_TAGLINE,
        ] {
            assert!(!t.is_empty());
        }
    }

    #[test]
    fn taglines_distinct() {
        let all = [
            COUDE_TAGLINE, COUDE_TAGLINE_SHORT, SENTINEL_TAGLINE,
            BLACKJACK_TAGLINE, SLOT_TAGLINE, WHEEL_TAGLINE,
        ];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j], "taglines {} et {} identiques", i, j);
            }
        }
    }

    #[test]
    fn coude_tagline_mentions_chaos() {
        assert!(COUDE_TAGLINE.to_lowercase().contains("chaos"));
        assert!(COUDE_TAGLINE_SHORT.to_lowercase().contains("chaos"));
    }
}

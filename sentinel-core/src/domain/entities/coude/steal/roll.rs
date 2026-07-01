//! Constants + helpers purs pour le RNG de /voler (Phase 2 #4 audit).
//!
//! Les vraies decisions (probabilite des d20, % de wallet volable) sont
//! ici pour pouvoir etre testees sans I/O. Le service API tire le RNG
//! uniformement et expose les valeurs au bot.

/// Borne basse du % de wallet vole en cas d'attaque sur cible AFK.
pub const STEAL_PCT_AFK_MIN_BP: u32 = 1000; // 10.00%
/// Borne haute du % de wallet vole en cas d'attaque sur cible AFK.
pub const STEAL_PCT_AFK_MAX_BP: u32 = 1500; // 15.00%
/// Borne basse du % de wallet vole en cas d'attaque sur cible defendue.
pub const STEAL_PCT_ACTIVE_MIN_BP: u32 = 1500; // 15.00%
/// Borne haute du % de wallet vole en cas d'attaque sur cible defendue.
pub const STEAL_PCT_ACTIVE_MAX_BP: u32 = 2500; // 25.00%

/// Min/max d'un d20 — exposes pour l'API et les tests.
pub const STEAL_D20_MIN: i32 = 1;
pub const STEAL_D20_MAX: i32 = 20;

use crate::domain::entities::coude::economy_config::CoudeEconomyConfig;

/// Plage en basis points (1bp = 0.01%) selon le statut AFK de la cible.
///
/// Les bornes proviennent de `cfg` (exprimées en POURCENTAGE côté config,
/// converties en basis points ici : bp = pct × 100). `cfg` garantit déjà
/// `min <= max` (cf. `CoudeEconomyConfig::sanitize`).
pub fn steal_pct_range_bp(afk: bool, cfg: &CoudeEconomyConfig) -> (u32, u32) {
    if afk {
        (cfg.steal_afk_min_pct * 100, cfg.steal_afk_max_pct * 100)
    } else {
        (
            cfg.steal_active_min_pct * 100,
            cfg.steal_active_max_pct * 100,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn afk_range_lower_than_active() {
        const _: () = assert!(STEAL_PCT_AFK_MAX_BP <= STEAL_PCT_ACTIVE_MIN_BP);
    }

    #[test]
    fn ranges_sane() {
        const _: () = assert!(STEAL_PCT_AFK_MIN_BP < STEAL_PCT_AFK_MAX_BP);
        const _: () = assert!(STEAL_PCT_ACTIVE_MIN_BP < STEAL_PCT_ACTIVE_MAX_BP);
        assert_eq!(STEAL_D20_MIN, 1);
        assert_eq!(STEAL_D20_MAX, 20);
    }

    #[test]
    fn afk_range_returns_afk_constants() {
        let (lo, hi) = steal_pct_range_bp(true, &CoudeEconomyConfig::default());
        assert_eq!(lo, STEAL_PCT_AFK_MIN_BP);
        assert_eq!(hi, STEAL_PCT_AFK_MAX_BP);
    }

    #[test]
    fn active_range_returns_active_constants() {
        let (lo, hi) = steal_pct_range_bp(false, &CoudeEconomyConfig::default());
        assert_eq!(lo, STEAL_PCT_ACTIVE_MIN_BP);
        assert_eq!(hi, STEAL_PCT_ACTIVE_MAX_BP);
    }

    #[test]
    fn custom_config_overrides_ranges() {
        let cfg = CoudeEconomyConfig {
            steal_afk_min_pct: 5,
            steal_afk_max_pct: 8,
            steal_active_min_pct: 20,
            steal_active_max_pct: 40,
            ..CoudeEconomyConfig::default()
        };
        assert_eq!(steal_pct_range_bp(true, &cfg), (500, 800));
        assert_eq!(steal_pct_range_bp(false, &cfg), (2000, 4000));
    }
}

/// Parse les multiplicateurs depuis le format config : "id:multiplier" par ligne.
pub fn parse_multipliers(raw: &str) -> Vec<(u64, f64)> {
    sentinel_shared::parsers::parse_id_f64_lines(raw)
        .into_iter()
        .filter(|(_, v)| *v > 0.0)
        .collect()
}

/// Retourne le multiplicateur pour un channel (defaut 1.0).
pub fn get_channel_multiplier(multipliers: &[(u64, f64)], channel_id: u64) -> f64 {
    sentinel_shared::parsers::lookup_f64(multipliers, channel_id, 1.0)
}

/// Retourne le meilleur multiplicateur parmi les roles de l'utilisateur (defaut 1.0).
pub fn get_role_multiplier(multipliers: &[(u64, f64)], user_roles: &[u64]) -> f64 {
    let mut best = 1.0f64;
    for role_id in user_roles {
        if let Some((_, mult)) = multipliers.iter().find(|(id, _)| id == role_id) {
            if *mult > best {
                best = *mult;
            }
        }
    }
    best
}

/// Calcul XP final unifié (texte + voice) appliquant base x channel x role x streak.
/// `clamp` permet d'eviter qu'un boost donne 0 (clamp_min=1) ou explose
/// (clamp_max). Quand seuls les multiplicateurs sont infos (voice ou
/// texte), passer `streak_mult = 1.0`.
///
/// Retourne un i64 borne aux limites de clamp.
pub fn calc_xp_amount(
    base_xp: f64,
    channel_mult: f64,
    role_mult: f64,
    streak_mult: f64,
    clamp_min: f64,
    clamp_max: f64,
) -> i64 {
    (base_xp * channel_mult * role_mult * streak_mult)
        .round()
        .clamp(clamp_min, clamp_max) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let raw = "123:2.0\n456:1.5";
        let mults = parse_multipliers(raw);
        assert_eq!(mults.len(), 2);
        assert_eq!(mults[0], (123, 2.0));
        assert_eq!(mults[1], (456, 1.5));
    }

    #[test]
    fn parse_ignores_empty() {
        let raw = "\n\n123:2.0\n\n";
        assert_eq!(parse_multipliers(raw).len(), 1);
    }

    #[test]
    fn parse_ignores_invalid() {
        let raw = "abc:2.0\n123:abc\n456:1.5";
        let mults = parse_multipliers(raw);
        assert_eq!(mults.len(), 1);
        assert_eq!(mults[0], (456, 1.5));
    }

    #[test]
    fn parse_ignores_zero_mult() {
        let raw = "123:0.0\n456:-1.0\n789:1.5";
        let mults = parse_multipliers(raw);
        assert_eq!(mults.len(), 1);
    }

    #[test]
    fn parse_empty() {
        assert!(parse_multipliers("").is_empty());
    }

    #[test]
    fn channel_mult_found() {
        let mults = vec![(123, 2.0), (456, 1.5)];
        assert_eq!(get_channel_multiplier(&mults, 123), 2.0);
    }

    #[test]
    fn channel_mult_default() {
        let mults = vec![(123, 2.0)];
        assert_eq!(get_channel_multiplier(&mults, 999), 1.0);
    }

    #[test]
    fn role_mult_best() {
        let mults = vec![(100, 1.2), (200, 1.5), (300, 1.3)];
        let user_roles = vec![100, 300];
        assert_eq!(get_role_multiplier(&mults, &user_roles), 1.3);
    }

    #[test]
    fn role_mult_no_match() {
        let mults = vec![(100, 2.0)];
        let user_roles = vec![999];
        assert_eq!(get_role_multiplier(&mults, &user_roles), 1.0);
    }

    #[test]
    fn role_mult_empty_roles() {
        let mults = vec![(100, 2.0)];
        assert_eq!(get_role_multiplier(&mults, &[]), 1.0);
    }

    // ── Tests calc_xp_amount : combinaisons x2 / x0.5 ──

    #[test]
    fn calc_neutral_returns_base() {
        // Aucun multiplicateur : XP = base.
        assert_eq!(calc_xp_amount(15.0, 1.0, 1.0, 1.0, 1.0, 1000.0), 15);
    }

    #[test]
    fn calc_channel_x2_doubles() {
        // Salon boost x2 sur un user normal => 30.
        assert_eq!(calc_xp_amount(15.0, 2.0, 1.0, 1.0, 1.0, 1000.0), 30);
    }

    #[test]
    fn calc_channel_x05_halves() {
        // Salon nerf x0.5 sur un user normal => 8 (15 * 0.5 = 7.5 -> round 8).
        assert_eq!(calc_xp_amount(15.0, 0.5, 1.0, 1.0, 1.0, 1000.0), 8);
    }

    #[test]
    fn calc_role_x2_doubles() {
        // VIP role x2 dans un salon normal => 30.
        assert_eq!(calc_xp_amount(15.0, 1.0, 2.0, 1.0, 1.0, 1000.0), 30);
    }

    #[test]
    fn calc_vip_in_nerf_channel_returns_normal() {
        // Logique demandee par l'utilisateur :
        // VIP (role x2) dans un salon nerf x0.5 => 15 * 0.5 * 2 = 15 (XP normal).
        assert_eq!(calc_xp_amount(15.0, 0.5, 2.0, 1.0, 1.0, 1000.0), 15);
    }

    #[test]
    fn calc_random_in_nerf_channel_halved() {
        // User normal dans un salon nerf x0.5 => 8.
        assert_eq!(calc_xp_amount(15.0, 0.5, 1.0, 1.0, 1.0, 1000.0), 8);
    }

    #[test]
    fn calc_vip_in_boost_channel_quadruples() {
        // VIP (role x2) dans un salon boost x2 => 60.
        assert_eq!(calc_xp_amount(15.0, 2.0, 2.0, 1.0, 1.0, 1000.0), 60);
    }

    #[test]
    fn calc_with_streak_compounds() {
        // VIP x2 dans salon x2 avec streak x1.5 => 15 * 2 * 2 * 1.5 = 90.
        assert_eq!(calc_xp_amount(15.0, 2.0, 2.0, 1.5, 1.0, 1000.0), 90);
    }

    #[test]
    fn calc_clamp_min_protects_against_zero() {
        // base * mult arrondi a 0 (mais clamp_min = 1) => 1, pas 0.
        // 0.49 * 1 * 1 * 1 = 0.49 -> round 0 -> clamp 1.
        assert_eq!(calc_xp_amount(0.49, 1.0, 1.0, 1.0, 1.0, 1000.0), 1);
    }

    #[test]
    fn calc_clamp_max_caps() {
        // 100 * 100 = 10000 -> clamp 1000 (max).
        assert_eq!(calc_xp_amount(100.0, 100.0, 1.0, 1.0, 1.0, 1000.0), 1000);
    }

    #[test]
    fn calc_voice_15_minutes_x2_channel() {
        // Scenario voice : 15 min en vocal x 5 xp/min = 75 base, salon x2 => 150.
        // (Le calcul reel utilise (seconds/60) * xp_per_min comme base_xp.)
        let base = 15.0 * 5.0; // 15 minutes * 5 XP/min
        assert_eq!(calc_xp_amount(base, 2.0, 1.0, 1.0, 1.0, 100_000.0), 150);
    }

    #[test]
    fn calc_voice_30_minutes_vip_in_x05_channel() {
        // 30 min, 5 XP/min = 150 base. VIP x2, salon x0.5 => 150 (annule).
        let base = 30.0 * 5.0;
        assert_eq!(calc_xp_amount(base, 0.5, 2.0, 1.0, 1.0, 100_000.0), 150);
    }

    #[test]
    fn calc_voice_random_in_x05_channel_halved() {
        // 30 min, 5 XP/min = 150 base. Salon x0.5 => 75.
        let base = 30.0 * 5.0;
        assert_eq!(calc_xp_amount(base, 0.5, 1.0, 1.0, 1.0, 100_000.0), 75);
    }
}

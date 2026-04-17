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
}

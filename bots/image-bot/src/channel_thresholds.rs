/// Parse les seuils par salon depuis le format config : "channel_id:threshold" par ligne.
pub fn parse_thresholds(raw: &str) -> Vec<(u64, f64)> {
    sentinel_shared::parsers::parse_id_f64_lines(raw)
        .into_iter()
        .filter(|(_, t)| (0.0..=1.0).contains(t))
        .collect()
}

/// Retourne le seuil configure pour un salon, ou le defaut.
pub fn get_channel_threshold(thresholds: &[(u64, f64)], channel_id: u64, default: f64) -> f64 {
    sentinel_shared::parsers::lookup_f64(thresholds, channel_id, default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let raw = "123:0.8\n456:0.6";
        let t = parse_thresholds(raw);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0], (123, 0.8));
        assert_eq!(t[1], (456, 0.6));
    }

    #[test]
    fn parse_ignores_empty() {
        let raw = "\n\n123:0.5\n\n";
        assert_eq!(parse_thresholds(raw).len(), 1);
    }

    #[test]
    fn parse_ignores_invalid() {
        let raw = "abc:0.5\n123:abc\n456:0.7";
        assert_eq!(parse_thresholds(raw).len(), 1);
    }

    #[test]
    fn parse_ignores_out_of_range() {
        let raw = "123:1.5\n456:-0.1\n789:0.5";
        assert_eq!(parse_thresholds(raw).len(), 1);
    }

    #[test]
    fn parse_empty() {
        assert!(parse_thresholds("").is_empty());
    }

    #[test]
    fn get_threshold_found() {
        let t = vec![(123, 0.8), (456, 0.6)];
        assert_eq!(get_channel_threshold(&t, 123, 0.5), 0.8);
    }

    #[test]
    fn get_threshold_default() {
        let t = vec![(123, 0.8)];
        assert_eq!(get_channel_threshold(&t, 999, 0.5), 0.5);
    }

    #[test]
    fn get_threshold_empty() {
        assert_eq!(get_channel_threshold(&[], 123, 0.5), 0.5);
    }

    #[test]
    fn parse_boundary_values() {
        let raw = "1:0.0\n2:1.0";
        let t = parse_thresholds(raw);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].1, 0.0);
        assert_eq!(t[1].1, 1.0);
    }
}

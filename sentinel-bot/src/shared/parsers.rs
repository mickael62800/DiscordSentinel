/// Parsers generiques pour les configurations "key separator value par ligne".
/// Utilises par tous les bots qui ont des configs texte multi-lignes.

/// Parse des lignes "label|value" (separateur pipe).
/// Ignore les lignes vides, sans pipe, ou avec label/value vide.
pub fn parse_pipe_lines(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() { return None; }
            let (left, right) = line.split_once('|')?;
            let left = left.trim();
            let right = right.trim();
            if left.is_empty() || right.is_empty() { return None; }
            Some((left.to_string(), right.to_string()))
        })
        .collect()
}

/// Parse des lignes "id:value" ou id est un u64 et value est un f64.
/// Ignore les lignes invalides.
pub fn parse_id_f64_lines(raw: &str) -> Vec<(u64, f64)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() { return None; }
            let (id_str, val_str) = line.split_once(':')?;
            let id: u64 = id_str.trim().parse().ok()?;
            let val: f64 = val_str.trim().parse().ok()?;
            Some((id, val))
        })
        .collect()
}

/// Parse des lignes "id:value" ou id est un u64 et value est un u64.
pub fn parse_id_u64_lines(raw: &str) -> Vec<(u64, u64)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() { return None; }
            let (id_str, val_str) = line.split_once(':')?;
            let id: u64 = id_str.trim().parse().ok()?;
            let val: u64 = val_str.trim().parse().ok()?;
            Some((id, val))
        })
        .collect()
}

/// Decoupe une chaine CSV en Vec<String> (trim + lowercase, ignore les vides).
pub fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Lookup dans un Vec<(u64, f64)> par id, retourne le defaut si non trouve.
pub fn lookup_f64(entries: &[(u64, f64)], id: u64, default: f64) -> f64 {
    entries.iter().find(|(k, _)| *k == id).map(|(_, v)| *v).unwrap_or(default)
}

/// Lookup dans un Vec<(u64, u64)> par id.
pub fn lookup_u64(entries: &[(u64, u64)], id: u64) -> Option<u64> {
    entries.iter().find(|(k, _)| *k == id).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_simple() {
        let r = parse_pipe_lines("A|B\nC|D");
        assert_eq!(r, vec![("A".into(), "B".into()), ("C".into(), "D".into())]);
    }

    #[test]
    fn pipe_ignores_empty() {
        assert_eq!(parse_pipe_lines("\n\nA|B\n\n").len(), 1);
    }

    #[test]
    fn pipe_ignores_invalid() {
        assert_eq!(parse_pipe_lines("no sep\n|b\na|\nOK|V").len(), 1);
    }

    #[test]
    fn pipe_trims() {
        let r = parse_pipe_lines("  X  |  Y  ");
        assert_eq!(r[0], ("X".into(), "Y".into()));
    }

    #[test]
    fn id_f64_simple() {
        let r = parse_id_f64_lines("123:2.0\n456:1.5");
        assert_eq!(r, vec![(123, 2.0), (456, 1.5)]);
    }

    #[test]
    fn id_f64_ignores_invalid() {
        assert_eq!(parse_id_f64_lines("abc:1.0\n123:abc\n456:1.5").len(), 1);
    }

    #[test]
    fn id_u64_simple() {
        let r = parse_id_u64_lines("111:3600\n222:86400");
        assert_eq!(r, vec![(111, 3600), (222, 86400)]);
    }

    #[test]
    fn csv_simple() {
        let r = split_csv("a, B , c");
        assert_eq!(r, vec!["a", "b", "c"]);
    }

    #[test]
    fn csv_empty() {
        assert!(split_csv("").is_empty());
    }

    #[test]
    fn lookup_f64_found() {
        assert_eq!(lookup_f64(&[(1, 2.0)], 1, 1.0), 2.0);
    }

    #[test]
    fn lookup_f64_default() {
        assert_eq!(lookup_f64(&[(1, 2.0)], 99, 1.0), 1.0);
    }

    #[test]
    fn lookup_u64_found() {
        assert_eq!(lookup_u64(&[(1, 100)], 1), Some(100));
    }

    #[test]
    fn lookup_u64_none() {
        assert_eq!(lookup_u64(&[(1, 100)], 99), None);
    }
}

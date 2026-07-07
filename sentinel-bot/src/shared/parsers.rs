/// Parsers generiques pour les configurations "key separator value par ligne".
/// Utilises par tous les bots qui ont des configs texte multi-lignes.

/// Parse des lignes "label|value" (separateur pipe).
/// Ignore les lignes vides, sans pipe, ou avec label/value vide.
pub fn parse_pipe_lines(raw: &str) -> Vec<(String, String)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (left, right) = line.split_once('|')?;
            let left = left.trim();
            let right = right.trim();
            if left.is_empty() || right.is_empty() {
                return None;
            }
            Some((left.to_string(), right.to_string()))
        })
        .collect()
}

/// Parse des lignes "id:value" ou id est un u64 et value est un u64.
pub fn parse_id_u64_lines(raw: &str) -> Vec<(u64, u64)> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
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

/// Lookup dans un Vec<(u64, u64)> par id.
pub fn lookup_u64(entries: &[(u64, u64)], id: u64) -> Option<u64> {
    entries.iter().find(|(k, _)| *k == id).map(|(_, v)| *v)
}

/// Formate le temps restant jusqu'a un instant RFC3339 sous forme d'echelle
/// `Xj Yh` / `Xh Ym` / `Xm` / `<1m`. Retourne `None` si le parsing echoue.
pub fn format_duration_remaining(expires_at_rfc3339: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(expires_at_rfc3339)
        .ok()
        .map(|expires| {
            let remaining = expires
                .with_timezone(&chrono::Utc)
                .signed_duration_since(chrono::Utc::now());
            if remaining.num_days() >= 1 {
                format!("{}j {}h", remaining.num_days(), remaining.num_hours() % 24)
            } else if remaining.num_hours() >= 1 {
                format!(
                    "{}h {}m",
                    remaining.num_hours(),
                    remaining.num_minutes() % 60
                )
            } else if remaining.num_minutes() >= 1 {
                format!("{}m", remaining.num_minutes())
            } else {
                "<1m".to_string()
            }
        })
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
    fn lookup_u64_found() {
        assert_eq!(lookup_u64(&[(1, 100)], 1), Some(100));
    }

    #[test]
    fn lookup_u64_none() {
        assert_eq!(lookup_u64(&[(1, 100)], 99), None);
    }

    #[test]
    fn duration_remaining_invalid_parse() {
        assert_eq!(format_duration_remaining("pas une date"), None);
    }

    #[test]
    fn duration_remaining_days() {
        // Buffer de 30min pour absorber la troncature (now() avance pendant le calcul).
        let future =
            (chrono::Utc::now() + chrono::Duration::hours(50) + chrono::Duration::minutes(30))
                .to_rfc3339();
        assert_eq!(format_duration_remaining(&future).as_deref(), Some("2j 2h"));
    }

    #[test]
    fn duration_remaining_hours() {
        let future = (chrono::Utc::now()
            + chrono::Duration::minutes(3 * 60 + 20)
            + chrono::Duration::seconds(30))
        .to_rfc3339();
        assert_eq!(
            format_duration_remaining(&future).as_deref(),
            Some("3h 20m")
        );
    }

    #[test]
    fn duration_remaining_minutes() {
        let future =
            (chrono::Utc::now() + chrono::Duration::minutes(45) + chrono::Duration::seconds(30))
                .to_rfc3339();
        assert_eq!(format_duration_remaining(&future).as_deref(), Some("45m"));
    }

    #[test]
    fn duration_remaining_past_is_under_one_minute() {
        let past = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        assert_eq!(format_duration_remaining(&past).as_deref(), Some("<1m"));
    }
}

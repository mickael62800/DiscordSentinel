/// Prerequis pour obtenir un role.
#[derive(Debug, Clone, PartialEq)]
pub enum Prerequisite {
    RequiresRole(u64),
    MinDays(u64),
}

/// Parse les prerequis depuis le format config.
/// Formats supportes par ligne :
/// - `role_id:requires_role:other_role_id`
/// - `role_id:min_days:N`
pub fn parse_prerequisites(raw: &str) -> Vec<(u64, Vec<Prerequisite>)> {
    let mut result: Vec<(u64, Vec<Prerequisite>)> = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() < 3 { continue; }

        let role_id: u64 = match parts[0].trim().parse() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let prereq = match parts[1].trim() {
            "requires_role" => {
                match parts[2].trim().parse::<u64>() {
                    Ok(rid) => Prerequisite::RequiresRole(rid),
                    Err(_) => continue,
                }
            }
            "min_days" => {
                match parts[2].trim().parse::<u64>() {
                    Ok(d) => Prerequisite::MinDays(d),
                    Err(_) => continue,
                }
            }
            _ => continue,
        };

        if let Some(entry) = result.iter_mut().find(|(id, _)| *id == role_id) {
            entry.1.push(prereq);
        } else {
            result.push((role_id, vec![prereq]));
        }
    }

    result
}

/// Verifie si les prerequis sont remplis pour un role.
/// Retourne Ok(()) si oui, Err(message) si non.
pub fn check_prerequisites(
    prereqs: &[(u64, Vec<Prerequisite>)],
    role_id: u64,
    user_roles: &[u64],
    joined_days: u64,
) -> Result<(), String> {
    let entry = match prereqs.iter().find(|(id, _)| *id == role_id) {
        Some(e) => e,
        None => return Ok(()), // Pas de prerequis
    };

    for prereq in &entry.1 {
        match prereq {
            Prerequisite::RequiresRole(required_role) => {
                if !user_roles.contains(required_role) {
                    return Err(format!("Vous devez avoir le role <@&{}> pour obtenir ce role.", required_role));
                }
            }
            Prerequisite::MinDays(min) => {
                if joined_days < *min {
                    return Err(format!(
                        "Vous devez etre dans le serveur depuis au moins {} jours (actuellement {} jours).",
                        min, joined_days
                    ));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_role() {
        let raw = "100:requires_role:200";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].0, 100);
        assert_eq!(prereqs[0].1, vec![Prerequisite::RequiresRole(200)]);
    }

    #[test]
    fn parse_min_days() {
        let raw = "100:min_days:30";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs[0].1, vec![Prerequisite::MinDays(30)]);
    }

    #[test]
    fn parse_multiple_for_same_role() {
        let raw = "100:requires_role:200\n100:min_days:7";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].1.len(), 2);
    }

    #[test]
    fn parse_multiple_roles() {
        let raw = "100:requires_role:200\n300:min_days:7";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs.len(), 2);
    }

    #[test]
    fn parse_ignores_invalid() {
        let raw = "invalid\n100:unknown:x\n200:requires_role:300";
        let prereqs = parse_prerequisites(raw);
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].0, 200);
    }

    #[test]
    fn parse_empty() {
        assert!(parse_prerequisites("").is_empty());
    }

    #[test]
    fn check_no_prereqs_passes() {
        let prereqs = vec![];
        assert!(check_prerequisites(&prereqs, 100, &[], 0).is_ok());
    }

    #[test]
    fn check_requires_role_passes() {
        let prereqs = vec![(100, vec![Prerequisite::RequiresRole(200)])];
        assert!(check_prerequisites(&prereqs, 100, &[200, 300], 0).is_ok());
    }

    #[test]
    fn check_requires_role_fails() {
        let prereqs = vec![(100, vec![Prerequisite::RequiresRole(200)])];
        let result = check_prerequisites(&prereqs, 100, &[300], 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("200"));
    }

    #[test]
    fn check_min_days_passes() {
        let prereqs = vec![(100, vec![Prerequisite::MinDays(30)])];
        assert!(check_prerequisites(&prereqs, 100, &[], 30).is_ok());
    }

    #[test]
    fn check_min_days_fails() {
        let prereqs = vec![(100, vec![Prerequisite::MinDays(30)])];
        let result = check_prerequisites(&prereqs, 100, &[], 10);
        assert!(result.is_err());
    }

    #[test]
    fn check_multiple_prereqs_all_pass() {
        let prereqs = vec![(100, vec![
            Prerequisite::RequiresRole(200),
            Prerequisite::MinDays(7),
        ])];
        assert!(check_prerequisites(&prereqs, 100, &[200], 10).is_ok());
    }

    #[test]
    fn check_multiple_prereqs_one_fails() {
        let prereqs = vec![(100, vec![
            Prerequisite::RequiresRole(200),
            Prerequisite::MinDays(30),
        ])];
        let result = check_prerequisites(&prereqs, 100, &[200], 10);
        assert!(result.is_err());
    }

    #[test]
    fn check_role_not_in_prereqs_passes() {
        let prereqs = vec![(100, vec![Prerequisite::MinDays(30)])];
        // Role 999 n'a pas de prerequis
        assert!(check_prerequisites(&prereqs, 999, &[], 0).is_ok());
    }
}

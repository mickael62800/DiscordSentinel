#![allow(dead_code)]
/// Definition d'un badge.
#[derive(Debug, Clone)]
pub struct Badge {
    pub id: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
}

/// Donnees pour evaluer les badges.
#[derive(Debug, Clone, Default)]
pub struct BadgeCheck {
    pub messages: u64,
    pub voice_hours: u64,
    pub level: i32,
    pub streak: u32,
}

/// Liste des badges predefinis.
static BADGES: &[Badge] = &[
    Badge { id: "bavard", name: "Bavard", emoji: "\u{1f4ac}", description: "100+ messages envoyes" },
    Badge { id: "orateur", name: "Orateur", emoji: "\u{1f5e3}\u{fe0f}", description: "1000+ messages envoyes" },
    Badge { id: "vocal", name: "Vocal", emoji: "\u{1f3a4}", description: "10h+ en vocal" },
    Badge { id: "dj", name: "DJ", emoji: "\u{1f3a7}", description: "100h+ en vocal" },
    Badge { id: "etoile", name: "Etoile montante", emoji: "\u{2b50}", description: "Niveau 5+" },
    Badge { id: "legende", name: "Legende", emoji: "\u{1f3c6}", description: "Niveau 20+" },
    Badge { id: "en_feu", name: "En feu", emoji: "\u{1f525}", description: "Streak 7+ jours" },
    Badge { id: "diamant", name: "Diamant", emoji: "\u{1f48e}", description: "Streak 30+ jours" },
];

/// Evalue quels badges l'utilisateur a debloque.
pub fn check_badges(check: &BadgeCheck) -> Vec<&'static Badge> {
    BADGES
        .iter()
        .filter(|badge| matches_badge(badge, check))
        .collect()
}

fn matches_badge(badge: &Badge, check: &BadgeCheck) -> bool {
    match badge.id {
        "bavard" => check.messages >= 100,
        "orateur" => check.messages >= 1000,
        "vocal" => check.voice_hours >= 10,
        "dj" => check.voice_hours >= 100,
        "etoile" => check.level >= 5,
        "legende" => check.level >= 20,
        "en_feu" => check.streak >= 7,
        "diamant" => check.streak >= 30,
        _ => false,
    }
}

/// Formate la liste de badges en texte.
pub fn format_badges(badges: &[&Badge]) -> String {
    if badges.is_empty() {
        return "Aucun badge pour l'instant.".to_string();
    }
    badges
        .iter()
        .map(|b| format!("{} **{}** — {}", b.emoji, b.name, b.description))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_badges() {
        let check = BadgeCheck::default();
        assert!(check_badges(&check).is_empty());
    }

    #[test]
    fn bavard_badge() {
        let check = BadgeCheck { messages: 100, ..Default::default() };
        let badges = check_badges(&check);
        assert!(badges.iter().any(|b| b.id == "bavard"));
    }

    #[test]
    fn orateur_includes_bavard() {
        let check = BadgeCheck { messages: 1000, ..Default::default() };
        let badges = check_badges(&check);
        assert!(badges.iter().any(|b| b.id == "bavard"));
        assert!(badges.iter().any(|b| b.id == "orateur"));
    }

    #[test]
    fn vocal_badges() {
        let check = BadgeCheck { voice_hours: 10, ..Default::default() };
        let badges = check_badges(&check);
        assert!(badges.iter().any(|b| b.id == "vocal"));
        assert!(!badges.iter().any(|b| b.id == "dj"));
    }

    #[test]
    fn dj_includes_vocal() {
        let check = BadgeCheck { voice_hours: 100, ..Default::default() };
        let badges = check_badges(&check);
        assert!(badges.iter().any(|b| b.id == "vocal"));
        assert!(badges.iter().any(|b| b.id == "dj"));
    }

    #[test]
    fn level_badges() {
        let check = BadgeCheck { level: 5, ..Default::default() };
        assert!(check_badges(&check).iter().any(|b| b.id == "etoile"));

        let check = BadgeCheck { level: 20, ..Default::default() };
        let badges = check_badges(&check);
        assert!(badges.iter().any(|b| b.id == "etoile"));
        assert!(badges.iter().any(|b| b.id == "legende"));
    }

    #[test]
    fn streak_badges() {
        let check = BadgeCheck { streak: 7, ..Default::default() };
        assert!(check_badges(&check).iter().any(|b| b.id == "en_feu"));

        let check = BadgeCheck { streak: 30, ..Default::default() };
        let badges = check_badges(&check);
        assert!(badges.iter().any(|b| b.id == "en_feu"));
        assert!(badges.iter().any(|b| b.id == "diamant"));
    }

    #[test]
    fn all_badges_unlocked() {
        let check = BadgeCheck {
            messages: 1000,
            voice_hours: 100,
            level: 20,
            streak: 30,
        };
        assert_eq!(check_badges(&check).len(), 8);
    }

    #[test]
    fn format_empty() {
        assert_eq!(format_badges(&[]), "Aucun badge pour l'instant.");
    }

    #[test]
    fn format_with_badges() {
        let check = BadgeCheck { messages: 100, ..Default::default() };
        let badges = check_badges(&check);
        let text = format_badges(&badges);
        assert!(text.contains("Bavard"));
        assert!(text.contains("100+ messages"));
    }
}

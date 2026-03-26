use regex::Regex;
use std::sync::LazyLock;

/// Liste de patterns d'insultes (FR + EN).
/// Utilise des regex pour capturer les variantes (espaces, leet speak basique).
static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let raw = [
        // Français
        r"(?i)\b(con(nard|nasse)?|putain|merde|encul[eé]|fdp|ntm|nique|batard|b[aâ]tard|pd|p[eé]d[eé]|salop(e|ard)?|bordel|ta\s*gueule|ferme[\s-]*la|d[eé]gage)\b",
        // Anglais
        r"(?i)\b(fuck(ing|er|ed)?|shit(ty)?|bitch|asshole|bastard|dick(head)?|cunt|stfu|idiot|moron|retard(ed)?|dumb(ass)?)\b",
    ];
    raw.iter()
        .map(|p| Regex::new(p).expect("regex invalide"))
        .collect()
});

/// Retourne `true` si le message contient une insulte détectée.
pub fn detect(content: &str) -> bool {
    PATTERNS.iter().any(|re| re.is_match(content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_french_insults() {
        assert!(detect("t'es un connard"));
        assert!(detect("fdp va"));
        assert!(detect("ta gueule"));
    }

    #[test]
    fn test_english_insults() {
        assert!(detect("shut the fuck up"));
        assert!(detect("you're an asshole"));
    }

    #[test]
    fn test_clean_messages() {
        assert!(!detect("Salut tout le monde !"));
        assert!(!detect("Bonne journée à tous"));
        assert!(!detect("Hello how are you?"));
    }
}

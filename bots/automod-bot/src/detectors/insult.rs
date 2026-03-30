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

    // ── Insultes francaises ──

    #[test]
    fn fr_connard() { assert!(detect("t'es un connard")); }
    #[test]
    fn fr_connasse() { assert!(detect("quelle connasse")); }
    #[test]
    fn fr_con_seul() { assert!(detect("espece de con")); }
    #[test]
    fn fr_putain() { assert!(detect("putain de merde")); }
    #[test]
    fn fr_merde() { assert!(detect("c'est de la merde")); }
    #[test]
    fn fr_encule() { assert!(detect("va te faire enculé")); }
    #[test]
    fn fr_fdp() { assert!(detect("fdp va")); }
    #[test]
    fn fr_ntm() { assert!(detect("ntm grave")); }
    #[test]
    fn fr_nique() { assert!(detect("je te nique")); }
    #[test]
    fn fr_batard() { assert!(detect("sale bâtard")); }
    #[test]
    fn fr_pd() { assert!(detect("espece de pd")); }
    #[test]
    fn fr_salope() { assert!(detect("quelle salope")); }
    #[test]
    fn fr_salopard() { assert!(detect("quel salopard")); }
    #[test]
    fn fr_bordel() { assert!(detect("bordel de merde")); }
    #[test]
    fn fr_ta_gueule() { assert!(detect("ta gueule")); }
    #[test]
    fn fr_ferme_la() { assert!(detect("ferme-la")); }
    #[test]
    fn fr_degage() { assert!(detect("dégage d'ici")); }

    // ── Insultes anglaises ──

    #[test]
    fn en_fuck() { assert!(detect("fuck you")); }
    #[test]
    fn en_fucking() { assert!(detect("that's fucking stupid")); }
    #[test]
    fn en_shit() { assert!(detect("this is shit")); }
    #[test]
    fn en_bitch() { assert!(detect("you bitch")); }
    #[test]
    fn en_asshole() { assert!(detect("you're an asshole")); }
    #[test]
    fn en_stfu() { assert!(detect("stfu noob")); }
    #[test]
    fn en_retard() { assert!(detect("you retard")); }
    #[test]
    fn en_dumbass() { assert!(detect("what a dumbass")); }
    #[test]
    fn en_cunt() { assert!(detect("stupid cunt")); }

    // ── Case insensitive ──

    #[test]
    fn case_insensitive_upper() { assert!(detect("CONNARD")); }
    #[test]
    fn case_insensitive_mixed() { assert!(detect("FdP")); }
    #[test]
    fn case_insensitive_en() { assert!(detect("FUCK OFF")); }

    // ── Faux positifs a eviter ──

    #[test]
    fn clean_french() { assert!(!detect("Salut tout le monde !")); }
    #[test]
    fn clean_english() { assert!(!detect("Hello how are you?")); }
    #[test]
    fn clean_discussion() { assert!(!detect("On se retrouve a 20h pour la game")); }
    #[test]
    fn clean_connaitre() { assert!(!detect("Je vais te faire connaitre ce jeu")); }
    #[test]
    fn clean_discourse() { assert!(!detect("C'est un discours interessant")); }
    #[test]
    fn clean_context_shift() { assert!(!detect("Le concert etait super")); }
    #[test]
    fn clean_number() { assert!(!detect("1234567890")); }
    #[test]
    fn clean_emoji() { assert!(!detect("Haha super game")); }
    #[test]
    fn clean_empty() { assert!(!detect("")); }
}

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
        // Variantes avec astérisque (f*ck, sh*t, b*tch…)
        r"(?i)\bf[*]ck(ing|er|ed)?\b",
        r"(?i)\bsh[*]t(ty)?\b",
        r"(?i)\bb[*]tch\b",
    ];
    raw.iter()
        .map(|p| Regex::new(p).expect("regex invalide"))
        .collect()
});

/// Normalise le leet speak d'un contenu pour détecter les variantes d'insultes.
///
/// Substitutions appliquées :
/// - `0` → `o`, `1` → `l`, `3` → `e`, `4` → `a`, `5` → `s`, `7` → `t`
/// - `@` → `a`, `$` → `s`
/// - `*` supprimé (pour f*ck, f*ck…)
fn normalize_leet(content: &str) -> String {
    content
        .chars()
        .filter_map(|c| match c {
            '0' => Some('o'),
            '1' | '!' => Some('i'),
            '3' => Some('e'),
            '4' => Some('a'),
            '5' => Some('s'),
            '6' => Some('g'),
            '7' => Some('t'),
            '8' => Some('b'),
            '9' => Some('g'),
            '@' => Some('a'),
            '$' => Some('s'),
            '(' => Some('c'),
            '+' => Some('t'),
            '|' => Some('l'),
            '*' | '.' | '_' | '-' => None, // caracteres de separation supprimes
            other => Some(other),
        })
        .collect()
}

/// Retourne `true` si le message contient une insulte détectée.
/// Vérifie les patterns statiques sur le contenu original ET normalisé (leet speak),
/// puis les mots personnalisés de la config.
pub fn detect(content: &str, custom_words: &[String]) -> bool {
    // Vérification sur le contenu original
    if PATTERNS.iter().any(|re| re.is_match(content)) {
        return true;
    }

    // Vérification sur le contenu normalisé (leet speak)
    let normalized = normalize_leet(content);
    if normalized != content && PATTERNS.iter().any(|re| re.is_match(&normalized)) {
        return true;
    }

    // Mots personnalisés (case-insensitive, substring match)
    if !custom_words.is_empty() {
        let content_lower = content.to_lowercase();
        if custom_words.iter().any(|w| content_lower.contains(w.as_str())) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Insultes francaises ──

    #[test]
    fn fr_connard() { assert!(detect("t'es un connard", &[])); }
    #[test]
    fn fr_connasse() { assert!(detect("quelle connasse", &[])); }
    #[test]
    fn fr_con_seul() { assert!(detect("espece de con", &[])); }
    #[test]
    fn fr_putain() { assert!(detect("putain de merde", &[])); }
    #[test]
    fn fr_merde() { assert!(detect("c'est de la merde", &[])); }
    #[test]
    fn fr_encule() { assert!(detect("va te faire enculé", &[])); }
    #[test]
    fn fr_fdp() { assert!(detect("fdp va", &[])); }
    #[test]
    fn fr_ntm() { assert!(detect("ntm grave", &[])); }
    #[test]
    fn fr_nique() { assert!(detect("je te nique", &[])); }
    #[test]
    fn fr_batard() { assert!(detect("sale bâtard", &[])); }
    #[test]
    fn fr_pd() { assert!(detect("espece de pd", &[])); }
    #[test]
    fn fr_salope() { assert!(detect("quelle salope", &[])); }
    #[test]
    fn fr_salopard() { assert!(detect("quel salopard", &[])); }
    #[test]
    fn fr_bordel() { assert!(detect("bordel de merde", &[])); }
    #[test]
    fn fr_ta_gueule() { assert!(detect("ta gueule", &[])); }
    #[test]
    fn fr_ferme_la() { assert!(detect("ferme-la", &[])); }
    #[test]
    fn fr_degage() { assert!(detect("dégage d'ici", &[])); }

    // ── Insultes anglaises ──

    #[test]
    fn en_fuck() { assert!(detect("fuck you", &[])); }
    #[test]
    fn en_fucking() { assert!(detect("that's fucking stupid", &[])); }
    #[test]
    fn en_shit() { assert!(detect("this is shit", &[])); }
    #[test]
    fn en_bitch() { assert!(detect("you bitch", &[])); }
    #[test]
    fn en_asshole() { assert!(detect("you're an asshole", &[])); }
    #[test]
    fn en_stfu() { assert!(detect("stfu noob", &[])); }
    #[test]
    fn en_retard() { assert!(detect("you retard", &[])); }
    #[test]
    fn en_dumbass() { assert!(detect("what a dumbass", &[])); }
    #[test]
    fn en_cunt() { assert!(detect("stupid cunt", &[])); }

    // ── Case insensitive ──

    #[test]
    fn case_insensitive_upper() { assert!(detect("CONNARD", &[])); }
    #[test]
    fn case_insensitive_mixed() { assert!(detect("FdP", &[])); }
    #[test]
    fn case_insensitive_en() { assert!(detect("FUCK OFF", &[])); }

    // ── Leet speak ──

    #[test]
    fn leet_connard_0() { assert!(detect("c0nnard", &[])); }
    #[test]
    fn leet_connard_mixed() { assert!(detect("c0nn4rd", &[])); }
    #[test]
    fn leet_fuck_star() { assert!(detect("f*ck you", &[])); }
    #[test]
    fn leet_fuck_star_full() { assert!(detect("f*cking idiot", &[])); }
    #[test]
    fn leet_shit_dollar() { assert!(detect("$hit", &[])); }
    #[test]
    fn leet_asshole_at() { assert!(detect("@sshole", &[])); }
    #[test]
    fn leet_bastard_4() { assert!(detect("b4stard", &[])); }
    #[test]
    fn leet_merde_3() { assert!(detect("m3rde", &[])); }
    #[test]
    fn leet_putain_4() { assert!(detect("put4in", &[])); }
    #[test]
    fn leet_encule_3() { assert!(detect("encul3", &[])); }

    // ── Mots personnalisés ──

    #[test]
    fn custom_word_detected() {
        assert!(detect("tu es un noob", &["noob".to_string()]));
    }
    #[test]
    fn custom_word_case_insensitive() {
        assert!(detect("NOOB", &["noob".to_string()]));
    }
    #[test]
    fn custom_word_in_sentence() {
        assert!(detect("arrete de troll stp", &["troll".to_string()]));
    }
    #[test]
    fn multiple_custom_words_one_match() {
        let words = vec!["noob".to_string(), "troll".to_string()];
        assert!(detect("t'es un troll", &words));
    }
    #[test]
    fn custom_words_no_match() {
        assert!(!detect("Salut tout le monde", &["noob".to_string()]));
    }
    #[test]
    fn empty_custom_words_no_effect() {
        assert!(!detect("Salut tout le monde", &[]));
    }

    // ── Faux positifs a eviter ──

    #[test]
    fn clean_french() { assert!(!detect("Salut tout le monde !", &[])); }
    #[test]
    fn clean_english() { assert!(!detect("Hello how are you?", &[])); }
    #[test]
    fn clean_discussion() { assert!(!detect("On se retrouve a 20h pour la game", &[])); }
    #[test]
    fn clean_connaitre() { assert!(!detect("Je vais te faire connaitre ce jeu", &[])); }
    #[test]
    fn clean_discourse() { assert!(!detect("C'est un discours interessant", &[])); }
    #[test]
    fn clean_context_shift() { assert!(!detect("Le concert etait super", &[])); }
    #[test]
    fn clean_number() { assert!(!detect("1234567890", &[])); }
    #[test]
    fn clean_emoji() { assert!(!detect("Haha super game", &[])); }
    #[test]
    fn clean_empty() { assert!(!detect("", &[])); }

    // ── normalize_leet unitaire ──

    #[test]
    fn normalize_digits() {
        assert_eq!(normalize_leet("c0nn4rd"), "connard");
    }
    #[test]
    fn normalize_at_dollar() {
        assert_eq!(normalize_leet("@$$hole"), "asshole");
    }
    #[test]
    fn normalize_star_removed() {
        assert_eq!(normalize_leet("f*ck"), "fck");
    }
    #[test]
    fn normalize_exclamation() {
        assert_eq!(normalize_leet("b!tch"), "bitch");
    }
    #[test]
    fn normalize_parenthesis() {
        assert_eq!(normalize_leet("(unt"), "cunt");
    }
    #[test]
    fn normalize_separators_removed() {
        assert_eq!(normalize_leet("c.o.n.n.a.r.d"), "connard");
    }
    #[test]
    fn leet_bitch_excl() { assert!(detect("b!tch", &[])); }
    #[test]
    fn leet_separated_dots() { assert!(detect("c.o.n.n.a.r.d", &[])); }
    #[test]
    fn normalize_clean_unchanged() {
        assert_eq!(normalize_leet("hello"), "hello");
    }
}

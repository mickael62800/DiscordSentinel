//! Règles de nommage Discord. Deux slugifieurs coexistent volontairement dans
//! le repo, avec des contraintes Discord distinctes :
//! - `slugify_channel_name` (ici) : noms de SALON — séparateur `-`, 90 chars.
//! - `slugify_emoji_name` (`entities/casino/game.rs`) : noms d'ÉMOJI —
//!   séparateur `_`, 32 chars, minimum 2.

/// Nettoie un nom pour en faire un nom de salon Discord valide (texte).
pub fn slugify_channel_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(90)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_and_dashes() {
        assert_eq!(slugify_channel_name("Mon Serveur FR!"), "mon-serveur-fr");
    }

    #[test]
    fn trims_leading_trailing_dashes() {
        assert_eq!(slugify_channel_name("--abc--"), "abc");
        assert_eq!(slugify_channel_name("!!!"), "");
    }

    #[test]
    fn caps_at_90_chars() {
        let long = "a".repeat(120);
        assert_eq!(slugify_channel_name(&long).chars().count(), 90);
    }

    #[test]
    fn keeps_unicode_alphanumerics() {
        assert_eq!(slugify_channel_name("Café été"), "café-été");
    }
}

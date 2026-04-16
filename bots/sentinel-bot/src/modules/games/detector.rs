use regex::Regex;
use std::sync::LazyLock;

/// Regex pour detecter les mentions de jeux : #NomDuJeu.
/// Formats supportes :
/// - #Fortnite           (un seul mot)
/// - #"Arc Riders"       (multi-mots entre guillemets)
/// - #"Baldur's Gate 3"  (avec apostrophes et chiffres)
static GAME_MENTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"#"([^"]+)"|#([\w]+)"#).unwrap()
});

/// Extrait les noms de jeux mentionnes dans un message.
/// Retourne les noms trimes et en minuscule pour la recherche.
pub fn extract_game_mentions(content: &str) -> Vec<String> {
    GAME_MENTION_RE
        .captures_iter(content)
        .filter_map(|cap| {
            cap.get(1) // "nom entre guillemets"
                .or(cap.get(2)) // #Mot1 Mot2
                .or(cap.get(3)) // #mot
                .map(|m| m.as_str().trim().to_string())
        })
        .filter(|name| !name.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_mention() {
        let mentions = extract_game_mentions("Qui veut jouer a #Fortnite ?");
        assert_eq!(mentions, vec!["Fortnite"]);
    }

    #[test]
    fn multi_word_quoted() {
        let mentions = extract_game_mentions(r#"On lance #"Arc Riders" ce soir"#);
        assert_eq!(mentions, vec!["Arc Riders"]);
    }

    #[test]
    fn quoted_with_special_chars() {
        let mentions = extract_game_mentions(r#"Qui pour #"Counter Strike 2" ?"#);
        assert_eq!(mentions, vec!["Counter Strike 2"]);
    }

    #[test]
    fn multiple_mentions() {
        let mentions = extract_game_mentions("#Fortnite ou #Valorant ?");
        assert_eq!(mentions.len(), 2);
        assert!(mentions.contains(&"Fortnite".to_string()));
        assert!(mentions.contains(&"Valorant".to_string()));
    }

    #[test]
    fn no_mention() {
        let mentions = extract_game_mentions("Salut tout le monde !");
        assert!(mentions.is_empty());
    }

    #[test]
    fn empty_message() {
        let mentions = extract_game_mentions("");
        assert!(mentions.is_empty());
    }

    #[test]
    fn hashtag_alone_ignored() {
        let mentions = extract_game_mentions("# ");
        assert!(mentions.is_empty());
    }

    #[test]
    fn quoted_with_apostrophe() {
        let mentions = extract_game_mentions(r#"#"Baldur's Gate 3""#);
        assert_eq!(mentions, vec!["Baldur's Gate 3"]);
    }

    #[test]
    fn mention_at_end_of_message() {
        let mentions = extract_game_mentions("Dispo pour #Fortnite");
        assert_eq!(mentions, vec!["Fortnite"]);
    }

    #[test]
    fn mixed_quoted_and_simple() {
        let mentions = extract_game_mentions(r#"On joue a #"Arc Riders" ou #Valorant ?"#);
        assert_eq!(mentions.len(), 2);
        assert!(mentions.contains(&"Arc Riders".to_string()));
        assert!(mentions.contains(&"Valorant".to_string()));
    }
}

/// Detection de spam par contenu :
/// - Repetition excessive de caracteres (aaaaaaa)
/// - Repetition de mots (buy buy buy buy buy)
/// Note : le flood (messages rapides) est gere dans le handler, pas ici.
pub fn detect(content: &str) -> bool {
    let trimmed = content.trim();

    if trimmed.len() < 2 {
        return false;
    }

    // Repetition de caracteres (ex: "aaaaaaa", "!!!!!!")
    let chars: Vec<char> = trimmed.chars().collect();
    let mut repeat_count = 1;
    for i in 1..chars.len() {
        if chars[i] == chars[i - 1] {
            repeat_count += 1;
            if repeat_count >= 6 {
                return true;
            }
        } else {
            repeat_count = 1;
        }
    }

    // Repetition de mots (ex: "buy buy buy buy buy")
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() >= 5 {
        let first = words[0].to_lowercase();
        if words.iter().all(|w| w.to_lowercase() == first) {
            return true;
        }
    }

    false
}

/// Detection de message tout en majuscules (>= 8 chars alphabetiques).
/// Ce n'est pas du spam, juste un avertissement.
pub fn detect_caps(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.len() >= 8
        && trimmed == trimmed.to_uppercase()
        && trimmed.chars().any(|c| c.is_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caps_not_spam() {
        assert!(!detect("ACHETE MON PRODUIT MAINTENANT"));
        assert!(detect_caps("ACHETE MON PRODUIT MAINTENANT"));
        assert!(!detect_caps("SALUT"));
    }

    #[test]
    fn test_char_repeat() {
        assert!(detect("aaaaaaa"));
        assert!(detect("hello!!!!!!"));
        assert!(!detect("hello!"));
    }

    #[test]
    fn test_word_repeat() {
        assert!(detect("buy buy buy buy buy"));
        assert!(!detect("buy something nice today"));
    }

    #[test]
    fn test_normal_message() {
        assert!(!detect("Salut, comment ça va ?"));
    }
}

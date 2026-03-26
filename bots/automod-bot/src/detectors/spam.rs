/// Détection basique de spam :
/// - Messages tout en majuscules (> 8 chars)
/// - Répétition excessive de caractères (aaaaaaa)
/// - Répétition de mots
pub fn detect(content: &str) -> bool {
    let trimmed = content.trim();

    if trimmed.len() < 2 {
        return false;
    }

    // Tout en majuscules (minimum 8 caractères)
    if trimmed.len() >= 8 && trimmed == trimmed.to_uppercase() && trimmed.chars().any(|c| c.is_alphabetic()) {
        return true;
    }

    // Répétition de caractères (ex: "aaaaaaa", "!!!!!!")
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

    // Répétition de mots (ex: "buy buy buy buy buy")
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() >= 5 {
        let first = words[0].to_lowercase();
        if words.iter().all(|w| w.to_lowercase() == first) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caps_spam() {
        assert!(detect("ACHETE MON PRODUIT MAINTENANT"));
        assert!(!detect("OK"));
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

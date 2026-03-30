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

    // ── Repetition de caracteres ──

    #[test]
    fn char_repeat_6_triggers() {
        assert!(detect("aaaaaa"));
    }

    #[test]
    fn char_repeat_5_does_not_trigger() {
        assert!(!detect("aaaaa"));
    }

    #[test]
    fn char_repeat_in_middle_of_text() {
        assert!(detect("hello!!!!!!world"));
        assert!(detect("salut aaaaaaa toi"));
    }

    #[test]
    fn char_repeat_question_marks() {
        assert!(detect("quoi ??????"));
    }

    #[test]
    fn char_repeat_mixed_no_trigger() {
        assert!(!detect("abcabc"));
        assert!(!detect("aabbcc"));
    }

    #[test]
    fn char_repeat_emoji_like() {
        assert!(!detect("haha"));
        assert!(!detect("lolol"));
    }

    // ── Repetition de mots ──

    #[test]
    fn word_repeat_5_triggers() {
        assert!(detect("buy buy buy buy buy"));
    }

    #[test]
    fn word_repeat_4_does_not_trigger() {
        assert!(!detect("buy buy buy buy"));
    }

    #[test]
    fn word_repeat_case_insensitive() {
        assert!(detect("SPAM Spam spam SpAm sPAM"));
    }

    #[test]
    fn word_repeat_different_words_no_trigger() {
        assert!(!detect("buy sell trade hold wait"));
    }

    #[test]
    fn word_repeat_with_same_punctuation_triggers() {
        // "ok!" repete 5x = meme token → spam
        assert!(detect("ok! ok! ok! ok! ok!"));
    }

    #[test]
    fn word_repeat_mixed_punctuation_no_trigger() {
        // Ponctuation differente = tokens differents
        assert!(!detect("ok! ok? ok. ok, ok;"));
    }

    // ── Caps detection ──

    #[test]
    fn caps_long_message_triggers() {
        assert!(detect_caps("ACHETE MON PRODUIT MAINTENANT"));
    }

    #[test]
    fn caps_short_message_no_trigger() {
        assert!(!detect_caps("SALUT"));
        assert!(!detect_caps("OK COOL"));
    }

    #[test]
    fn caps_exactly_8_chars_triggers() {
        assert!(detect_caps("ABCDEFGH"));
    }

    #[test]
    fn caps_7_chars_no_trigger() {
        assert!(!detect_caps("ABCDEFG"));
    }

    #[test]
    fn caps_numbers_only_no_trigger() {
        assert!(!detect_caps("12345678"));
    }

    #[test]
    fn caps_mixed_case_no_trigger() {
        assert!(!detect_caps("Salut Comment Ca Va"));
    }

    #[test]
    fn caps_symbols_only_no_trigger() {
        assert!(!detect_caps("!!!!!!!!!!"));
    }

    #[test]
    fn caps_with_numbers_triggers() {
        assert!(detect_caps("ALERTE 123 URGENT"));
    }

    // ── Messages normaux ──

    #[test]
    fn normal_french_message() {
        assert!(!detect("Salut, comment ca va ?"));
        assert!(!detect_caps("Salut, comment ca va ?"));
    }

    #[test]
    fn empty_message() {
        assert!(!detect(""));
        assert!(!detect_caps(""));
    }

    #[test]
    fn single_char() {
        assert!(!detect("a"));
        assert!(!detect_caps("A"));
    }

    #[test]
    fn whitespace_only() {
        assert!(!detect("   "));
        assert!(!detect_caps("   "));
    }

    #[test]
    fn normal_long_message() {
        assert!(!detect("Bonjour tout le monde, je suis nouveau sur le serveur et je cherche des gens pour jouer"));
    }
}

use regex::Regex;
use std::sync::LazyLock;

/// Regex pour détecter les URLs (http, https, discord invites).
static URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(https?://\S+|discord\.gg/\S+|discord\.com/invite/\S+)").expect("regex invalide")
});

/// Retourne `true` si le message contient un lien.
pub fn detect(content: &str) -> bool {
    URL_PATTERN.is_match(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_links() {
        assert!(detect("Va voir https://example.com"));
        assert!(detect("http://malware.xyz/payload"));
    }

    #[test]
    fn test_discord_invites() {
        assert!(detect("Rejoins discord.gg/abc123"));
        assert!(detect("discord.com/invite/test"));
    }

    #[test]
    fn test_no_links() {
        assert!(!detect("Salut tout le monde"));
        assert!(!detect("Mon site est example.com")); // pas de protocole
    }
}

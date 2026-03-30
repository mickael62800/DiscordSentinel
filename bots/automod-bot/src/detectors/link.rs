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

    // ── URLs avec protocole ──

    #[test]
    fn https_simple() { assert!(detect("https://example.com")); }
    #[test]
    fn http_simple() { assert!(detect("http://example.com")); }
    #[test]
    fn https_with_path() { assert!(detect("https://example.com/page/sub?q=1")); }
    #[test]
    fn url_in_text() { assert!(detect("Va voir https://example.com pour plus d'infos")); }
    #[test]
    fn multiple_urls() { assert!(detect("https://a.com et https://b.com")); }

    // ── Discord invites ──

    #[test]
    fn discord_gg() { assert!(detect("Rejoins discord.gg/abc123")); }
    #[test]
    fn discord_com_invite() { assert!(detect("discord.com/invite/test")); }
    #[test]
    fn discord_gg_in_text() { assert!(detect("Mon serveur discord.gg/monserv venez")); }

    // ── Pas de lien ──

    #[test]
    fn no_protocol() { assert!(!detect("Mon site est example.com")); }
    #[test]
    fn clean_text() { assert!(!detect("Salut tout le monde")); }
    #[test]
    fn empty() { assert!(!detect("")); }
    #[test]
    fn email_not_url() { assert!(!detect("contact@example.com")); }
    #[test]
    fn dotted_words() { assert!(!detect("e.g. c'est a dire")); }
    #[test]
    fn ip_without_protocol() { assert!(!detect("192.168.1.1")); }
    #[test]
    fn ip_with_protocol() { assert!(detect("http://192.168.1.1/admin")); }
}

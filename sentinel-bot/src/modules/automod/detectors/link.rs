use regex::Regex;
use std::sync::LazyLock;

/// Regex pour détecter les URLs (http, https, discord invites).
static URL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(https?://\S+|discord\.gg/\S+|discord\.com/invite/\S+)")
        .expect("regex invalide")
});

/// Retourne `true` si le message contient un lien non autorisé.
///
/// - `allow_discord_invites` : si true, les liens discord.gg/* et discord.com/invite/* sont ignorés.
/// - `allowed_domains` : liste de domaines autorisés (ex: ["twitch.tv", "youtube.com"]).
///   Un URL contenant l'un de ces domaines n'est pas flagué.
pub fn detect(content: &str, allow_discord_invites: bool, allowed_domains: &[String]) -> bool {
    for m in URL_PATTERN.find_iter(content) {
        let url = m.as_str().to_lowercase();

        if allow_discord_invites && is_discord_invite(&url) {
            continue;
        }

        if allowed_domains.iter().any(|d| url.contains(d.as_str())) {
            continue;
        }

        return true;
    }
    false
}

fn is_discord_invite(url: &str) -> bool {
    url.contains("discord.gg/") || url.contains("discord.com/invite/")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── URLs avec protocole ──

    #[test]
    fn https_simple() {
        assert!(detect("https://example.com", false, &[]));
    }
    #[test]
    fn http_simple() {
        assert!(detect("http://example.com", false, &[]));
    }
    #[test]
    fn https_with_path() {
        assert!(detect("https://example.com/page/sub?q=1", false, &[]));
    }
    #[test]
    fn url_in_text() {
        assert!(detect(
            "Va voir https://example.com pour plus d'infos",
            false,
            &[]
        ));
    }
    #[test]
    fn multiple_urls() {
        assert!(detect("https://a.com et https://b.com", false, &[]));
    }

    // ── Discord invites ──

    #[test]
    fn discord_gg_blocked_by_default() {
        assert!(detect("Rejoins discord.gg/abc123", false, &[]));
    }
    #[test]
    fn discord_com_invite_blocked_by_default() {
        assert!(detect("discord.com/invite/test", false, &[]));
    }

    #[test]
    fn discord_gg_allowed_when_configured() {
        assert!(!detect("Rejoins discord.gg/abc123", true, &[]));
    }
    #[test]
    fn discord_com_invite_allowed_when_configured() {
        assert!(!detect("discord.com/invite/test", true, &[]));
    }
    #[test]
    fn discord_invite_in_text_allowed() {
        assert!(!detect("Mon serveur discord.gg/monserv venez", true, &[]));
    }

    // ── Domaines autorisés ──

    #[test]
    fn allowed_domain_not_flagged() {
        let allowed = vec!["twitch.tv".to_string()];
        assert!(!detect("https://twitch.tv/monstream", false, &allowed));
    }
    #[test]
    fn allowed_domain_case_insensitive() {
        let allowed = vec!["youtube.com".to_string()];
        assert!(!detect("https://YOUTUBE.COM/watch?v=abc", false, &allowed));
    }
    #[test]
    fn non_allowed_domain_still_flagged() {
        let allowed = vec!["twitch.tv".to_string()];
        assert!(detect("https://badsite.com/hack", false, &allowed));
    }
    #[test]
    fn multiple_urls_one_allowed_one_not() {
        let allowed = vec!["twitch.tv".to_string()];
        // https://badsite.com n'est pas autorisé → true
        assert!(detect(
            "https://twitch.tv/ok et https://badsite.com/hack",
            false,
            &allowed
        ));
    }
    #[test]
    fn all_urls_allowed() {
        let allowed = vec!["twitch.tv".to_string(), "youtube.com".to_string()];
        assert!(!detect(
            "https://twitch.tv/ok https://youtube.com/watch?v=abc",
            false,
            &allowed
        ));
    }

    // ── Pas de lien ──

    #[test]
    fn no_protocol() {
        assert!(!detect("Mon site est example.com", false, &[]));
    }
    #[test]
    fn clean_text() {
        assert!(!detect("Salut tout le monde", false, &[]));
    }
    #[test]
    fn empty() {
        assert!(!detect("", false, &[]));
    }
    #[test]
    fn email_not_url() {
        assert!(!detect("contact@example.com", false, &[]));
    }
    #[test]
    fn dotted_words() {
        assert!(!detect("e.g. c'est a dire", false, &[]));
    }
    #[test]
    fn ip_without_protocol() {
        assert!(!detect("192.168.1.1", false, &[]));
    }
    #[test]
    fn ip_with_protocol() {
        assert!(detect("http://192.168.1.1/admin", false, &[]));
    }
}

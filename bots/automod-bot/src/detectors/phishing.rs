use regex::Regex;
use std::sync::LazyLock;

/// Domaines de phishing connus et patterns de scam Discord.
static PHISHING_DOMAINS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)https?://(",
        // Typosquatting Discord
        r"d[il1]sc[o0]rd[\w-]*\.(gift|com|gg|app|click|ru|xyz|top|info|net|org|co)",
        r"|disc[o0]rd[\w-]*app\.\w+",
        r"|discordnitro[\w-]*\.\w+",
        r"|discord-[\w-]*\.(com|gift|click|ru|xyz|top)",
        // Typosquatting Steam
        r"|st[e3][a@]m[\w-]*community[\w-]*\.\w+",
        r"|steam[\w-]*pow[e3]r[\w-]*\.\w+",
        r"|steamcommunlty\.\w+",
        r"|steampowored\.\w+",
        // Faux cadeaux / airdrops
        r"|[\w-]*free[\w-]*nitro[\w-]*\.\w+",
        r"|[\w-]*nitro[\w-]*gift[\w-]*\.\w+",
        r"|[\w-]*crypto[\w-]*airdrop[\w-]*\.\w+",
        // IP grabbers connus
        r"|grabify\.link",
        r"|iplogger\.\w+",
        r"|blasze\.tk",
        r"|2no\.co",
        // Raccourcisseurs suspects
        r"|bit\.do",
        r"|cutt\.ly[\w/]*",
        // Phishing generique
        r"|[\w-]*login[\w-]*verify[\w-]*\.\w+",
        r"|[\w-]*verify[\w-]*account[\w-]*\.\w+",
        ")/\\S*",
    ))
    .expect("regex phishing invalide")
});

/// Patterns de messages scam classiques.
static SCAM_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let raw = [
        // Faux cadeaux Nitro
        r"(?i)(free|gratuit)\s+(discord\s+)?nitro",
        r"(?i)discord\s*nitro\s*(for\s+)?free",
        r"(?i)(recois|claim|reclame)\s+(ton|your)\s+(cadeau|gift|nitro)",
        // Faux cadeaux Steam
        r"(?i)(free|gratuit)\s+steam\s+(gift|game|wallet|card)",
        r"(?i)steam\s+(gift|game|wallet)\s*(for\s+)?free",
        // Crypto scam
        r"(?i)(earn|gagne[rz]?)\s+\$?\d+[\w\s]*crypto",
        r"(?i)(bitcoin|ethereum|crypto)\s+(giveaway|airdrop|doubl)",
        r"(?i)send\s+\d[\d.]*\s*(btc|eth)\s*(and|et)\s*(get|receive|recois)",
        // DM scam classiques
        r"(?i)(your|ton|votre)\s+account\s+(has\s+been|will\s+be)\s+(disabled|suspended|banned|terminated)",
        r"(?i)(verify|confirm)\s+(your|ton)\s+(account|identity)\s+(before|within)\s+\d+\s*(hours?|heures?)",
        // QR code scam
        r"(?i)scan\s+(this|ce)\s+qr\s*code",
    ];
    raw.iter()
        .map(|p| Regex::new(p).expect("regex scam invalide"))
        .collect()
});

/// Domaines legitimes a ne pas flaguer.
static LEGITIMATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)https?://(www\.)?(discord\.(com|gg|new)|store\.steampowered\.com|steamcommunity\.com|cdn\.discordapp\.com)/").expect("regex legitime invalide")
});

/// Retourne `true` si le message contient un lien de phishing ou un pattern scam.
pub fn detect(content: &str) -> bool {
    if SCAM_PATTERNS.iter().any(|re| re.is_match(content)) {
        return true;
    }
    // Verifier les domaines phishing en excluant les vrais domaines
    for m in PHISHING_DOMAINS.find_iter(content) {
        let url = m.as_str();
        if !LEGITIMATE.is_match(url) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_phishing_links() {
        assert!(detect("Va voir https://dlscord.gift/nitro-free"));
        assert!(detect("https://disc0rd-app.com/verify"));
        assert!(detect("https://discordnitro-gift.xyz/claim"));
        assert!(detect("Regarde https://discord-gift.ru/free"));
    }

    #[test]
    fn test_steam_phishing() {
        assert!(detect("https://steamcommunlty.com/trade"));
        assert!(detect("https://steampowored.com/login"));
    }

    #[test]
    fn test_ip_grabbers() {
        assert!(detect("https://grabify.link/abc123"));
        assert!(detect("https://iplogger.org/test"));
    }

    #[test]
    fn test_scam_messages() {
        assert!(detect("Free Discord Nitro! Claim now"));
        assert!(detect("Recois ton cadeau nitro gratuit"));
        assert!(detect("Free steam gift card for everyone"));
        assert!(detect("Earn $500 in crypto today"));
        assert!(detect("Send 0.1 BTC and get 1 BTC back"));
        assert!(detect("Your account has been disabled, verify within 24 hours"));
    }

    #[test]
    fn test_legitimate_messages() {
        assert!(!detect("Salut, tu veux jouer a un jeu ?"));
        assert!(!detect("J'ai achete Nitro hier c'est cool"));
        assert!(!detect("https://discord.com/channels/123/456"));
        assert!(!detect("https://store.steampowered.com/app/730"));
        assert!(!detect("Mon compte Steam est ancien"));
        assert!(!detect("J'ai de la crypto sur Binance"));
    }

    #[test]
    fn test_real_discord_links_not_flagged() {
        assert!(!detect("https://discord.com/invite/abc123"));
        assert!(!detect("https://discord.gg/serveur"));
    }
}

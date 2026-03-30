pub mod spam;
pub mod insult;
pub mod link;
pub mod phishing;

/// Résultat de l'analyse locale d'un message.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectionFlags {
    pub spam: bool,
    pub insult: bool,
    pub link: bool,
    pub phishing: bool,
}

/// Analyse un message et retourne les flags de détection.
pub fn analyze(content: &str) -> DetectionFlags {
    DetectionFlags {
        spam: spam::detect(content),
        insult: insult::detect(content),
        link: link::detect(content),
        phishing: phishing::detect(content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_message_no_flags() {
        let f = analyze("Salut, on fait une game ce soir ?");
        assert!(!f.spam && !f.insult && !f.link && !f.phishing);
    }

    #[test]
    fn spam_only() {
        let f = analyze("aaaaaaa");
        assert!(f.spam);
        assert!(!f.insult && !f.link && !f.phishing);
    }

    #[test]
    fn insult_only() {
        let f = analyze("t'es un connard");
        assert!(f.insult);
        assert!(!f.spam && !f.phishing);
    }

    #[test]
    fn link_only() {
        let f = analyze("Va voir https://example.com");
        assert!(f.link);
        assert!(!f.spam && !f.insult && !f.phishing);
    }

    #[test]
    fn phishing_detected() {
        let f = analyze("Free Discord Nitro click here");
        assert!(f.phishing);
    }

    #[test]
    fn insult_with_link() {
        let f = analyze("fdp regarde https://example.com");
        assert!(f.insult && f.link);
    }

    #[test]
    fn spam_with_insult() {
        let f = analyze("merde merde merde merde merde");
        assert!(f.spam && f.insult);
    }

    #[test]
    fn phishing_link_combo() {
        let f = analyze("https://dlscord.gift/free-nitro");
        assert!(f.link && f.phishing);
    }

    #[test]
    fn empty_message() {
        let f = analyze("");
        assert!(!f.spam && !f.insult && !f.link && !f.phishing);
    }
}

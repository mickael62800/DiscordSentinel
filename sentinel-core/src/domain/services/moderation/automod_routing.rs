//! Decision de routage automod (full hexa) : DECIDE = API.
//!
//! Centralise la regle "que faire d'une detection ?" (carte de review / action
//! auto / rien), auparavant dupliquee cote bot. Le bot n'a plus qu'a EXECUTER
//! la decision retournee. Fonction pure : aucune I/O, testable directement.

use crate::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::domain::enums::moderation::action::Action;

/// Que doit faire le bot de la detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Routing {
    /// Ne rien faire automatiquement (ex. human_only sans salon de review).
    None,
    /// Poster une carte de review/vote.
    Card,
    /// Appliquer directement l'action (mode auto, hors human_only).
    Auto,
}

/// Decision complete de routage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingDecision {
    pub route: Routing,
    /// Cas severe (phishing / invitation Discord) : protection auto immediate.
    pub severe: bool,
    /// Lien non autorise HORS image : suppression auto immediate.
    pub auto_delete_link: bool,
}

/// Entrees de la decision (faits + config guild deja resolue par l'API).
pub struct RoutingInputs<'a> {
    pub flags: &'a DetectionFlags,
    pub content: &'a str,
    pub score: f64,
    pub action: Action,
    pub human_only: bool,
    pub auto_protect: bool,
    pub auto_delete_links: bool,
    pub ai_review_mode: bool,
    pub review_min_score: f64,
    /// `true` si un salon de review est configure (log_channel_id != 0).
    pub log_channel_set: bool,
}

/// `true` si l'invitation Discord (pub vers un autre serveur) est presente.
pub fn contains_discord_invite(content: &str) -> bool {
    let l = content.to_lowercase();
    l.contains("discord.gg/")
        || l.contains("discord.com/invite/")
        || l.contains("discordapp.com/invite/")
}

/// Cas "severe" justifiant une protection auto immediate meme en human_only :
/// phishing/scam ou invitation Discord.
pub fn is_severe_content(flags: &DetectionFlags, content: &str) -> bool {
    flags.phishing || contains_discord_invite(content)
}

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "apng", "avif"];

/// `true` si le message contient au moins une URL http(s) qui n'est PAS une
/// image (lien "hors image" a supprimer).
pub fn contains_non_image_url(content: &str) -> bool {
    content.split_whitespace().any(|tok| {
        let t = tok.trim_matches(|c: char| {
            !c.is_ascii_alphanumeric() && c != '/' && c != ':' && c != '.' && c != '-' && c != '_'
        });
        let lower = t.to_lowercase();
        if !(lower.starts_with("http://") || lower.starts_with("https://")) {
            return false;
        }
        let path = lower.split(['?', '#']).next().unwrap_or(&lower);
        let ext = path.rsplit('.').next().unwrap_or("");
        !IMAGE_EXTS.contains(&ext)
    })
}

/// Calcule la decision de routage a partir des faits + config guild.
pub fn decide(i: &RoutingInputs) -> RoutingDecision {
    let severe = i.auto_protect && is_severe_content(i.flags, i.content);

    let auto_delete_link = !severe
        && i.auto_delete_links
        && i.flags.link
        && !i.flags.phishing
        && contains_non_image_url(i.content);

    let above_threshold = i.score >= i.review_min_score;
    let should_card =
        i.log_channel_set && (i.human_only || severe || (i.ai_review_mode && above_threshold));

    let route = if should_card {
        Routing::Card
    } else if i.human_only {
        // Pas de carte (pas de salon) + human_only : aucune action auto.
        Routing::None
    } else if matches!(i.action, Action::None) {
        Routing::None
    } else {
        Routing::Auto
    };

    RoutingDecision { route, severe, auto_delete_link }
}

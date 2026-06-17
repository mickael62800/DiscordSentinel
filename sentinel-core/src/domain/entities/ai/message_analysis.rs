use crate::domain::enums::moderation::action::Action;
use crate::domain::services::moderation::automod_routing::Routing;

/// Résultat de l'analyse d'un message par le domaine.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MessageAnalysis {
    pub action: Action,
    pub reason: String,
    pub score: f64,
    pub duration: Option<u64>,
    /// Décision de routage calculée côté serveur (decide = API).
    pub route: Routing,
    /// Cas sévère (phishing / invitation Discord) → protection auto.
    pub severe: bool,
    /// Lien non autorisé hors image → suppression auto.
    pub auto_delete_link: bool,
}

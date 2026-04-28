use crate::domain::enums::moderation::action::Action;

/// Résultat de l'analyse d'un message par le domaine.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MessageAnalysis {
    pub action: Action,
    pub reason: String,
    pub score: f64,
    pub duration: Option<u64>,
}

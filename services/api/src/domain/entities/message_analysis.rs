use crate::domain::value_objects::Action;

/// Résultat de l'analyse d'un message par le domaine.
#[derive(Debug, Clone)]
pub struct MessageAnalysis {
    pub action: Action,
    pub reason: String,
    pub score: f64,
    pub duration: Option<u64>,
}

use crate::domain::value_objects::Action;

/// Resultat de l'analyse d'une image par le modele vision ONNX.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ImageAnalysis {
    pub action: Action,
    pub reason: String,
    pub score: f64,
    pub duration: Option<u64>,
    pub classifications: Vec<ImageClassification>,
}

#[derive(Debug, Clone)]
pub struct ImageClassification {
    pub label: String,
    pub confidence: f32,
}

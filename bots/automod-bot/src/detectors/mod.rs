pub mod spam;
pub mod insult;
pub mod link;

/// Résultat de l'analyse locale d'un message.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectionFlags {
    pub spam: bool,
    pub insult: bool,
    pub link: bool,
}

/// Analyse un message et retourne les flags de détection.
pub fn analyze(content: &str) -> DetectionFlags {
    DetectionFlags {
        spam: spam::detect(content),
        insult: insult::detect(content),
        link: link::detect(content),
    }
}

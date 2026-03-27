use std::path::Path;
use std::sync::Mutex;

use ndarray::{Array2, Array4};
use ort::session::Session;
use ort::value::Value;
use tracing::{info, warn};

/// Classification produite par un modele ONNX.
#[derive(Debug, Clone)]
pub struct InferenceClassification {
    pub label: String,
    pub confidence: f32,
}

/// Service d'inference ONNX — charge les modeles au demarrage.
/// Les sessions sont protegees par Mutex car `session.run()` requiert `&mut`.
pub struct InferenceService {
    vision_session: Option<Mutex<Session>>,
    text_session: Option<Mutex<Session>>,
}

impl InferenceService {
    /// Charge les modeles ONNX depuis les chemins fournis.
    /// Si un modele n'est pas trouve, le service fonctionne en mode degrade.
    pub fn new(vision_model_path: Option<&str>, text_model_path: Option<&str>) -> Self {
        let vision_session = vision_model_path.and_then(|p| {
            if !Path::new(p).exists() {
                warn!(path = %p, "Modele vision ONNX introuvable — inference vision desactivee");
                return None;
            }
            let result = (|| -> Result<Session, Box<dyn std::error::Error>> {
                let builder = Session::builder()?;
                let mut builder = builder.with_intra_threads(4).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                let session = builder.commit_from_file(p)?;
                Ok(session)
            })();
            match result {
                Ok(session) => {
                    info!(path = %p, "Modele vision ONNX charge");
                    Some(Mutex::new(session))
                }
                Err(e) => {
                    warn!(error = %e, "Erreur chargement modele vision ONNX");
                    None
                }
            }
        });

        let text_session = text_model_path.and_then(|p| {
            if !Path::new(p).exists() {
                warn!(path = %p, "Modele text ONNX introuvable — inference text desactivee");
                return None;
            }
            let result = (|| -> Result<Session, Box<dyn std::error::Error>> {
                let builder = Session::builder()?;
                let mut builder = builder.with_intra_threads(4).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                let session = builder.commit_from_file(p)?;
                Ok(session)
            })();
            match result {
                Ok(session) => {
                    info!(path = %p, "Modele text ONNX charge");
                    Some(Mutex::new(session))
                }
                Err(e) => {
                    warn!(error = %e, "Erreur chargement modele text ONNX");
                    None
                }
            }
        });

        Self {
            vision_session,
            text_session,
        }
    }

    pub fn vision_available(&self) -> bool {
        self.vision_session.is_some()
    }

    pub fn text_available(&self) -> bool {
        self.text_session.is_some()
    }

    /// Inference vision : prend une image preprocessee (1, 3, 224, 224) normalisee.
    /// Retourne les classifications (safe, nsfw, illicit) avec confidences.
    pub fn classify_image(&self, image_tensor: Array4<f32>) -> Result<Vec<InferenceClassification>, String> {
        let mutex = self.vision_session.as_ref()
            .ok_or("Modele vision non charge")?;
        let mut session = mutex.lock()
            .map_err(|e| format!("Lock session vision: {e}"))?;

        let input = Value::from_array(image_tensor)
            .map_err(|e| format!("Erreur creation tensor: {e}"))?;

        let outputs = session.run(ort::inputs![input])
            .map_err(|e| format!("Erreur inference vision: {e}"))?;

        let output_view = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Erreur extraction output: {e}"))?;

        let logits = output_view.1;
        let probabilities = softmax(logits);

        let labels = ["safe", "nsfw", "illicit"];
        let classifications: Vec<InferenceClassification> = labels
            .iter()
            .zip(probabilities.iter())
            .map(|(label, &confidence)| InferenceClassification {
                label: label.to_string(),
                confidence,
            })
            .collect();

        Ok(classifications)
    }

    /// Inference text : prend des token IDs et un attention mask.
    /// Retourne les classifications (neutral, anger, rage, threat, harassment).
    pub fn classify_text(
        &self,
        input_ids: Array2<i64>,
        attention_mask: Array2<i64>,
    ) -> Result<Vec<InferenceClassification>, String> {
        let mutex = self.text_session.as_ref()
            .ok_or("Modele text non charge")?;
        let mut session = mutex.lock()
            .map_err(|e| format!("Lock session text: {e}"))?;

        let ids_value = Value::from_array(input_ids)
            .map_err(|e| format!("Erreur creation tensor input_ids: {e}"))?;
        let mask_value = Value::from_array(attention_mask)
            .map_err(|e| format!("Erreur creation tensor attention_mask: {e}"))?;

        let outputs = session.run(ort::inputs![ids_value, mask_value])
            .map_err(|e| format!("Erreur inference text: {e}"))?;

        let output_view = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Erreur extraction output: {e}"))?;

        let logits = output_view.1;
        let probabilities = softmax(logits);

        let labels = ["neutral", "anger", "rage", "threat", "harassment"];
        let classifications: Vec<InferenceClassification> = labels
            .iter()
            .zip(probabilities.iter())
            .map(|(label, &confidence)| InferenceClassification {
                label: label.to_string(),
                confidence,
            })
            .collect();

        Ok(classifications)
    }
}

/// Softmax sur un slice de logits.
fn softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|x| (x - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    exps.into_iter().map(|e| e / sum).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tests softmax ──

    #[test]
    fn test_softmax_sums_to_one() {
        let logits = vec![1.0, 2.0, 3.0];
        let probs = softmax(&logits);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_softmax_highest_logit_gets_highest_prob() {
        let logits = vec![1.0, 5.0, 2.0];
        let probs = softmax(&logits);
        assert!(probs[1] > probs[0]);
        assert!(probs[1] > probs[2]);
    }

    #[test]
    fn test_softmax_equal_logits_gives_uniform() {
        let logits = vec![2.0, 2.0, 2.0];
        let probs = softmax(&logits);
        for p in &probs {
            assert!((p - 1.0 / 3.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_softmax_single_element() {
        let probs = softmax(&[42.0]);
        assert_eq!(probs.len(), 1);
        assert!((probs[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_softmax_large_values_no_overflow() {
        let logits = vec![1000.0, 1001.0, 1002.0];
        let probs = softmax(&logits);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
        assert!(probs[2] > probs[1]);
    }

    #[test]
    fn test_softmax_negative_values() {
        let logits = vec![-1.0, -2.0, -3.0];
        let probs = softmax(&logits);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!(probs[0] > probs[1]);
        assert!(probs[1] > probs[2]);
    }

    // ── Tests InferenceService mode degrade ──

    #[test]
    fn test_inference_no_models_loaded() {
        let service = InferenceService::new(None, None);
        assert!(!service.vision_available());
        assert!(!service.text_available());
    }

    #[test]
    fn test_inference_nonexistent_paths() {
        let service = InferenceService::new(
            Some("/nonexistent/vision.onnx"),
            Some("/nonexistent/text.onnx"),
        );
        assert!(!service.vision_available());
        assert!(!service.text_available());
    }

    #[test]
    fn test_classify_image_without_model_returns_error() {
        let service = InferenceService::new(None, None);
        let tensor = Array4::<f32>::zeros((1, 3, 224, 224));
        let result = service.classify_image(tensor);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non charge"));
    }

    #[test]
    fn test_classify_text_without_model_returns_error() {
        let service = InferenceService::new(None, None);
        let ids = Array2::<i64>::zeros((1, 10));
        let mask = Array2::<i64>::ones((1, 10));
        let result = service.classify_text(ids, mask);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non charge"));
    }
}

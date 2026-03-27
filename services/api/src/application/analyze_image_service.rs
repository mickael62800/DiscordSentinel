use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

use crate::domain::entities::{ImageAnalysis, ImageClassification, Infraction};
use crate::domain::errors::DomainError;
use crate::domain::services::InferenceService;
use crate::domain::value_objects::{Action, DetectionFlags, FlagType};
use crate::ports::inbound::{AnalyzeImageCommand, AnalyzeImageUseCase, DeductPointsCommand, ManageConductUseCase};
use crate::ports::outbound::{CachePort, InfractionRepository, RuleRepository};

/// Seuil de confiance minimum pour considerer une classification IA comme positive.
const CONFIDENCE_THRESHOLD: f32 = 0.5;

pub struct AnalyzeImageService {
    inference: Arc<InferenceService>,
    rule_repo: Arc<dyn RuleRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
    conduct_uc: Arc<dyn ManageConductUseCase>,
}

impl AnalyzeImageService {
    pub fn new(
        inference: Arc<InferenceService>,
        rule_repo: Arc<dyn RuleRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
        conduct_uc: Arc<dyn ManageConductUseCase>,
    ) -> Self {
        Self {
            inference,
            rule_repo,
            infraction_repo,
            cache,
            conduct_uc,
        }
    }
}

#[async_trait]
impl AnalyzeImageUseCase for AnalyzeImageService {
    async fn analyze_image(&self, cmd: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> {
        // 1. Verifier que le modele vision est disponible
        if !self.inference.vision_available() {
            return Ok(ImageAnalysis {
                action: Action::None,
                reason: "Modele vision non disponible".to_string(),
                score: 0.0,
                duration: None,
                classifications: vec![],
            });
        }

        // 2. Preprocesser l'image (decode, resize, normalize)
        let image_tensor = preprocess_image(&cmd.image_bytes)
            .map_err(|e| DomainError::Internal(format!("Erreur preprocessing image: {e}")))?;

        // 3. Inference ONNX
        let classifications = self.inference.classify_image(image_tensor)
            .map_err(|e| DomainError::Internal(format!("Erreur inference: {e}")))?;

        info!(
            classifications = ?classifications.iter().map(|c| format!("{}:{:.2}", c.label, c.confidence)).collect::<Vec<_>>(),
            user = %cmd.username,
            "Resultat inference vision"
        );

        // 4. Convertir en DetectionFlags pour le scoring
        let flags = DetectionFlags {
            spam: false,
            insult: false,
            link: false,
            phishing: false,
        };

        let mut detected_labels = Vec::new();

        for c in &classifications {
            match c.label.as_str() {
                "nsfw" if c.confidence >= CONFIDENCE_THRESHOLD => {
                    detected_labels.push(FlagType::Nsfw);
                }
                "illicit" if c.confidence >= CONFIDENCE_THRESHOLD => {
                    detected_labels.push(FlagType::Illicit);
                }
                _ => {}
            }
        }

        if detected_labels.is_empty() {
            return Ok(ImageAnalysis {
                action: Action::None,
                reason: String::new(),
                score: 0.0,
                duration: None,
                classifications: classifications
                    .into_iter()
                    .map(|c| ImageClassification {
                        label: c.label,
                        confidence: c.confidence,
                    })
                    .collect(),
            });
        }

        // 5. Charger les regles et scorer
        let rules = match self.cache.get_rules(&cmd.guild_id).await? {
            Some(cached) => cached,
            None => {
                let from_db = self.rule_repo.find_by_guild(&cmd.guild_id).await?;
                self.cache.set_rules(&cmd.guild_id, &from_db).await.ok();
                from_db
            }
        };

        // Calculer le score manuellement avec les flags IA
        let mut total_score = 0.0;
        let mut triggered: Vec<&str> = Vec::new();

        for flag_type in &detected_labels {
            let rule = rules.iter().find(|r| r.flag_type == *flag_type && r.enabled);
            let weight = match rule {
                Some(r) => r.weight,
                None => match flag_type {
                    FlagType::Nsfw => 8.0,
                    FlagType::Illicit => 9.0,
                    _ => 5.0,
                },
            };
            total_score += weight;
            triggered.push(flag_type.as_str());
        }

        // Determiner l'action
        let (action, duration) = if total_score >= 9.0 {
            (Action::Ban, None)
        } else if total_score >= 6.0 {
            (Action::Mute, Some(600))
        } else if total_score >= 4.0 {
            (Action::Delete, None)
        } else if total_score >= 2.0 {
            (Action::Warn, None)
        } else {
            (Action::None, None)
        };

        let reason = format!(
            "Image detectee : {} (score: {:.1})",
            triggered.join(", "),
            total_score
        );

        // 6. Persister l'infraction
        // On utilise des flags factices pour le champ flags de Infraction
        // car le systeme actuel attend des DetectionFlags texte
        let infraction = Infraction {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            channel_id: cmd.channel_id,
            user_id: cmd.user_id.clone(),
            username: cmd.username.clone(),
            message_id: cmd.message_id,
            content: format!("[Image: {}]", cmd.filename),
            flags,
            score: total_score,
            action: action.clone(),
            reason: reason.clone(),
            duration,
            created_at: chrono::Utc::now(),
        };

        self.infraction_repo.save(&infraction).await?;

        // 6b. Deduire les points de conduite
        if action.as_str() != "none" {
            let _ = self.conduct_uc.deduct_points(DeductPointsCommand {
                guild_id: infraction.guild_id.clone(),
                user_id: infraction.user_id.clone(),
                username: infraction.username.clone(),
                action: action.as_str().to_string(),
            }).await;
        }

        // 7. Retourner le resultat
        Ok(ImageAnalysis {
            action,
            reason,
            score: total_score,
            duration,
            classifications: classifications
                .into_iter()
                .map(|c| ImageClassification {
                    label: c.label,
                    confidence: c.confidence,
                })
                .collect(),
        })
    }
}

/// Preprocesse une image brute en tensor (1, 3, 224, 224) normalise ImageNet.
fn preprocess_image(bytes: &[u8]) -> Result<ndarray::Array4<f32>, String> {
    use image::GenericImageView;

    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("Image invalide: {e}"))?;

    let resized = img.resize_exact(224, 224, image::imageops::FilterType::Triangle);

    let mut tensor = ndarray::Array4::<f32>::zeros((1, 3, 224, 224));

    // Normalisation ImageNet : mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]
    let mean = [0.485f32, 0.456, 0.406];
    let std = [0.229f32, 0.224, 0.225];

    for (x, y, pixel) in resized.pixels() {
        let rgb = pixel.0;
        for c in 0..3 {
            tensor[[0, c, y as usize, x as usize]] =
                (rgb[c] as f32 / 255.0 - mean[c]) / std[c];
        }
    }

    Ok(tensor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_invalid_bytes_returns_error() {
        let result = preprocess_image(&[0, 1, 2, 3]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Image invalide"));
    }

    #[test]
    fn test_preprocess_valid_png() {
        // Creer un PNG 2x2 minimal en memoire
        let mut buf = Vec::new();
        {
            use image::{ImageBuffer, Rgb};
            let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(2, 2, |_, _| {
                Rgb([128, 64, 200])
            });
            let mut cursor = std::io::Cursor::new(&mut buf);
            img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
        }

        let tensor = preprocess_image(&buf).unwrap();
        assert_eq!(tensor.shape(), &[1, 3, 224, 224]);
    }

    #[test]
    fn test_preprocess_normalization_range() {
        // Un pixel blanc (255, 255, 255) normalise : (1.0 - mean) / std
        // Channel R : (1.0 - 0.485) / 0.229 ≈ 2.249
        let mut buf = Vec::new();
        {
            use image::{ImageBuffer, Rgb};
            let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(1, 1, |_, _| {
                Rgb([255, 255, 255])
            });
            let mut cursor = std::io::Cursor::new(&mut buf);
            img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
        }

        let tensor = preprocess_image(&buf).unwrap();
        let val_r = tensor[[0, 0, 0, 0]];
        // (255/255 - 0.485) / 0.229 ≈ 2.249
        assert!((val_r - 2.249).abs() < 0.01);
    }

    #[test]
    fn test_preprocess_black_pixel_normalization() {
        let mut buf = Vec::new();
        {
            use image::{ImageBuffer, Rgb};
            let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(1, 1, |_, _| {
                Rgb([0, 0, 0])
            });
            let mut cursor = std::io::Cursor::new(&mut buf);
            img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
        }

        let tensor = preprocess_image(&buf).unwrap();
        let val_r = tensor[[0, 0, 0, 0]];
        // (0/255 - 0.485) / 0.229 ≈ -2.118
        assert!((val_r - (-2.118)).abs() < 0.01);
    }
}

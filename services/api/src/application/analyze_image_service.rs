use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;
use uuid::Uuid;

use crate::domain::entities::{ImageAnalysis, ImageClassification, Infraction};
use crate::domain::errors::DomainError;
use crate::domain::services::{InferenceRateLimiter, InferenceService};
use crate::domain::value_objects::{Action, DetectionFlags, FlagType};
use crate::ports::inbound::{AnalyzeImageCommand, AnalyzeImageUseCase, DeductPointsCommand, ManageConductUseCase};
use crate::ports::outbound::{CachePort, IaConfigRepository, InfractionRepository, RuleRepository};

/// Seuil de confiance par defaut (utilise si pas de config per-guild).
const DEFAULT_VISION_THRESHOLD: f32 = 0.5;

pub struct AnalyzeImageService {
    inference: Arc<InferenceService>,
    rule_repo: Arc<dyn RuleRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
    conduct_uc: Arc<dyn ManageConductUseCase>,
    ia_config_repo: Arc<dyn IaConfigRepository>,
    inference_limiter: Arc<InferenceRateLimiter>,
}

impl AnalyzeImageService {
    pub fn new(
        inference: Arc<InferenceService>,
        rule_repo: Arc<dyn RuleRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
        conduct_uc: Arc<dyn ManageConductUseCase>,
        ia_config_repo: Arc<dyn IaConfigRepository>,
        inference_limiter: Arc<InferenceRateLimiter>,
    ) -> Self {
        Self {
            inference,
            rule_repo,
            infraction_repo,
            cache,
            conduct_uc,
            ia_config_repo,
            inference_limiter,
        }
    }
}

#[async_trait]
impl AnalyzeImageUseCase for AnalyzeImageService {
    async fn analyze_image(&self, cmd: AnalyzeImageCommand) -> Result<ImageAnalysis, DomainError> {
        // 0. Charger la config IA per-guild
        let ia_config = match self.ia_config_repo.get(&cmd.guild_id).await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::warn!(error = %e, guild_id = %cmd.guild_id, "Echec chargement config IA vision, utilisation defauts");
                None
            }
        };
        let vision_enabled = ia_config.as_ref().map(|c| c.vision_enabled).unwrap_or(true);
        let vision_threshold = ia_config.as_ref().map(|c| c.vision_threshold as f32).unwrap_or(DEFAULT_VISION_THRESHOLD);

        // 1. Verifier que le modele vision est disponible et active
        if !vision_enabled || !self.inference.vision_available() {
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

        // 3. Inference ONNX (rate limited)
        let _permit = self.inference_limiter.acquire().await?;
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
                "nsfw" if c.confidence >= vision_threshold => {
                    detected_labels.push(FlagType::Nsfw);
                }
                "illicit" if c.confidence >= vision_threshold => {
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
                if let Err(e) = self.cache.set_rules(&cmd.guild_id, &from_db).await {
                    tracing::warn!(error = %e, guild_id = %cmd.guild_id, "Echec cache set rules (vision)");
                }
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

        // Seuils depuis les rules (configurables per-guild), pas hardcodes.
        let (t_warn, t_delete, t_mute, t_ban) =
            crate::domain::services::resolve_thresholds(&rules);
        let (action, duration) = if total_score >= t_ban {
            (Action::Ban, None)
        } else if total_score >= t_mute {
            (Action::Mute, Some(600))
        } else if total_score >= t_delete {
            (Action::Delete, None)
        } else if total_score >= t_warn {
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
            if let Err(e) = self.conduct_uc.deduct_points(DeductPointsCommand {
                guild_id: infraction.guild_id.clone(),
                user_id: infraction.user_id.clone(),
                username: infraction.username.clone(),
                action: action.as_str().to_string(),
            }).await {
                tracing::warn!(error = %e, guild_id = %infraction.guild_id, user_id = %infraction.user_id, "Echec deduction points conduite (analyse image)");
            }
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
#[path = "tests/analyze_image_service.rs"]
mod tests;

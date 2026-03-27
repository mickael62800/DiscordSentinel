use std::path::Path;

use ndarray::Array2;
use tokenizers::Tokenizer;
use tracing::{info, warn};

/// Wrapper autour du tokenizer HuggingFace pour preparer les inputs du modele text ONNX.
pub struct TextTokenizer {
    tokenizer: Option<Tokenizer>,
    max_length: usize,
}

impl TextTokenizer {
    /// Charge un tokenizer depuis un fichier tokenizer.json.
    /// Si le fichier n'existe pas, le tokenizer fonctionne en mode degrade (pas d'inference text).
    pub fn new(tokenizer_path: Option<&str>, max_length: usize) -> Self {
        let tokenizer = tokenizer_path.and_then(|p| {
            if !Path::new(p).exists() {
                warn!(path = %p, "Tokenizer introuvable — inference text desactivee");
                return None;
            }
            match Tokenizer::from_file(p) {
                Ok(mut tok) => {
                    // Configurer le padding et la troncature
                    let padding = tokenizers::PaddingParams {
                        strategy: tokenizers::PaddingStrategy::Fixed(max_length),
                        pad_id: 0,
                        pad_token: "[PAD]".to_string(),
                        ..Default::default()
                    };
                    tok.with_padding(Some(padding));

                    let truncation = tokenizers::TruncationParams {
                        max_length,
                        ..Default::default()
                    };
                    tok.with_truncation(Some(truncation)).ok();

                    info!(path = %p, max_length, "Tokenizer charge");
                    Some(tok)
                }
                Err(e) => {
                    warn!(error = %e, "Erreur chargement tokenizer");
                    None
                }
            }
        });

        Self {
            tokenizer,
            max_length,
        }
    }

    pub fn available(&self) -> bool {
        self.tokenizer.is_some()
    }

    /// Tokenise un texte et retourne (input_ids, attention_mask) prets pour ONNX.
    /// Shape : (1, max_length) pour les deux tensors.
    pub fn tokenize(&self, text: &str) -> Result<(Array2<i64>, Array2<i64>), String> {
        let tokenizer = self.tokenizer.as_ref()
            .ok_or("Tokenizer non charge")?;

        let encoding = tokenizer.encode(text, true)
            .map_err(|e| format!("Erreur tokenisation: {e}"))?;

        let ids = encoding.get_ids();
        let mask = encoding.get_attention_mask();

        // Convertir en ndarray (1, seq_len)
        let seq_len = ids.len().min(self.max_length);

        let input_ids = Array2::from_shape_fn((1, seq_len), |(_, j)| ids[j] as i64);
        let attention_mask = Array2::from_shape_fn((1, seq_len), |(_, j)| mask[j] as i64);

        Ok((input_ids, attention_mask))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_none_path_not_available() {
        let tok = TextTokenizer::new(None, 256);
        assert!(!tok.available());
    }

    #[test]
    fn test_tokenizer_nonexistent_path_not_available() {
        let tok = TextTokenizer::new(Some("/nonexistent/tokenizer.json"), 256);
        assert!(!tok.available());
    }

    #[test]
    fn test_tokenize_without_tokenizer_returns_error() {
        let tok = TextTokenizer::new(None, 256);
        let result = tok.tokenize("hello world");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non charge"));
    }

    #[test]
    fn test_max_length_stored() {
        let tok = TextTokenizer::new(None, 128);
        // max_length est correctement stocke meme sans tokenizer
        assert_eq!(tok.max_length, 128);
    }
}

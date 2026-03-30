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
                    // Detecter le pad token du modele (CamemBERT=<pad>/1, BERT=[PAD]/0)
                    let (pad_id, pad_token) = tok.get_vocab(true)
                        .iter()
                        .find(|(token, _)| *token == "<pad>" || *token == "[PAD]")
                        .map(|(token, &id)| (id, token.clone()))
                        .unwrap_or((0, "[PAD]".to_string()));

                    let padding = tokenizers::PaddingParams {
                        strategy: tokenizers::PaddingStrategy::Fixed(max_length),
                        pad_id,
                        pad_token,
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
        assert_eq!(tok.max_length, 128);
    }

    // ── Tests avec le vrai tokenizer ──

    const TOKENIZER_PATH: &str = "../../ai/training/text/exports/tokenizer.json";

    fn load_real_tokenizer() -> Option<TextTokenizer> {
        let tok = TextTokenizer::new(Some(TOKENIZER_PATH), 256);
        if tok.available() { Some(tok) } else { None }
    }

    #[test]
    fn real_tokenizer_loads_successfully() {
        let tok = load_real_tokenizer();
        assert!(tok.is_some(), "Tokenizer introuvable a {TOKENIZER_PATH}");
    }

    #[test]
    fn real_tokenizer_simple_text() {
        let Some(tok) = load_real_tokenizer() else { return };
        let (ids, mask) = tok.tokenize("Bonjour tout le monde").unwrap();
        assert_eq!(ids.shape(), &[1, 256]);
        assert_eq!(mask.shape(), &[1, 256]);
        // Les premiers tokens doivent etre non-zero (vrais tokens)
        assert_ne!(ids[[0, 0]], 0);
        // Le mask doit avoir des 1 au debut et des 0 a la fin (padding)
        assert_eq!(mask[[0, 0]], 1);
        assert_eq!(mask[[0, 255]], 0);
    }

    #[test]
    fn real_tokenizer_empty_text() {
        let Some(tok) = load_real_tokenizer() else { return };
        let (ids, mask) = tok.tokenize("").unwrap();
        assert_eq!(ids.shape(), &[1, 256]);
        // Meme un texte vide produit au moins les tokens speciaux (CLS, SEP)
        assert_eq!(mask[[0, 0]], 1);
    }

    #[test]
    fn real_tokenizer_long_text_truncated() {
        let Some(tok) = load_real_tokenizer() else { return };
        let long_text = "mot ".repeat(1000);
        let (ids, mask) = tok.tokenize(&long_text).unwrap();
        assert_eq!(ids.shape(), &[1, 256]);
        // Tous les slots doivent etre remplis (pas de padding)
        assert_eq!(mask[[0, 255]], 1);
    }

    #[test]
    fn real_tokenizer_special_chars() {
        let Some(tok) = load_real_tokenizer() else { return };
        let result = tok.tokenize("😡🤬💀 je vais te 💩 espèce de $#@!");
        assert!(result.is_ok());
    }

    #[test]
    fn real_tokenizer_french_insults() {
        let Some(tok) = load_real_tokenizer() else { return };
        let result = tok.tokenize("t'es qu'un connard, ferme ta gueule");
        assert!(result.is_ok());
        let (ids, _) = result.unwrap();
        // Les tokens doivent etre significatifs (pas tous padding)
        let non_zero = ids.iter().filter(|&&v| v != 0).count();
        assert!(non_zero > 3);
    }
}

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

// ── Tests du helper pur `build_arrays_from_encoding` ──

#[test]
fn build_arrays_short_sequence_truncated_to_len() {
    let (ids, mask) = build_arrays_from_encoding(&[1, 2, 3], &[1, 1, 1], 256);
    // seq_len = min(3, 256) = 3
    assert_eq!(ids.shape(), &[1, 3]);
    assert_eq!(mask.shape(), &[1, 3]);
    assert_eq!(ids[[0, 0]], 1);
    assert_eq!(ids[[0, 2]], 3);
    assert_eq!(mask[[0, 0]], 1);
}

#[test]
fn build_arrays_long_sequence_capped_to_max_length() {
    let ids: Vec<u32> = (0..500).collect();
    let mask: Vec<u32> = vec![1; 500];
    let (out_ids, out_mask) = build_arrays_from_encoding(&ids, &mask, 256);
    assert_eq!(out_ids.shape(), &[1, 256]);
    assert_eq!(out_mask.shape(), &[1, 256]);
    assert_eq!(out_ids[[0, 0]], 0);
    assert_eq!(out_ids[[0, 255]], 255);
}

#[test]
fn build_arrays_empty_sequence() {
    let (ids, mask) = build_arrays_from_encoding(&[], &[], 256);
    assert_eq!(ids.shape(), &[1, 0]);
    assert_eq!(mask.shape(), &[1, 0]);
}

#[test]
fn build_arrays_u32_max_cast_to_i64() {
    let (ids, _) = build_arrays_from_encoding(&[u32::MAX], &[1], 256);
    assert_eq!(ids[[0, 0]], u32::MAX as i64);
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
#[ignore = "Necessite le fichier tokenizer sur le disque"]
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
    assert_ne!(ids[[0, 0]], 0);
    assert_eq!(mask[[0, 0]], 1);
    assert_eq!(mask[[0, 255]], 0);
}

#[test]
fn real_tokenizer_empty_text() {
    let Some(tok) = load_real_tokenizer() else { return };
    let (ids, mask) = tok.tokenize("").unwrap();
    assert_eq!(ids.shape(), &[1, 256]);
    assert_eq!(mask[[0, 0]], 1);
    let _ = ids;
}

#[test]
fn real_tokenizer_long_text_truncated() {
    let Some(tok) = load_real_tokenizer() else { return };
    let long_text = "mot ".repeat(1000);
    let (ids, mask) = tok.tokenize(&long_text).unwrap();
    assert_eq!(ids.shape(), &[1, 256]);
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
    let non_zero = ids.iter().filter(|&&v| v != 0).count();
    assert!(non_zero > 3);
}

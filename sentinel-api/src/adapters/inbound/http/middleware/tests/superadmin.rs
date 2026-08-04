use super::*;

#[test]
fn token_cache_key_is_stable() {
    assert_eq!(token_cache_key("abc"), token_cache_key("abc"));
}

#[test]
fn token_cache_key_differs_per_token() {
    assert_ne!(token_cache_key("token-a"), token_cache_key("token-b"));
}

#[test]
fn token_cache_key_is_128_bits_hex() {
    let k = token_cache_key("whatever");
    assert_eq!(k.len(), 32);
    assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn token_cache_key_never_leaks_the_token() {
    let token = "super-secret-access-token";
    assert!(!token_cache_key(token).contains(token));
}

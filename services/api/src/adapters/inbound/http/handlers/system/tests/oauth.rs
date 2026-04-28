use super::*;

#[test]
fn percent_encode_preserves_unreserved() {
    assert_eq!(percent_encode("abcXYZ0123-._~"), "abcXYZ0123-._~");
}

#[test]
fn percent_encode_space_and_slash() {
    assert_eq!(percent_encode("hello world"), "hello%20world");
    assert_eq!(percent_encode("a/b"), "a%2Fb");
}

#[test]
fn percent_encode_non_ascii_as_utf8_bytes() {
    // "é" = 0xC3 0xA9 en UTF-8
    assert_eq!(percent_encode("é"), "%C3%A9");
}

#[test]
fn percent_encode_scopes_space() {
    assert_eq!(percent_encode("identify guilds"), "identify%20guilds");
}

#[test]
fn percent_encode_empty() {
    assert_eq!(percent_encode(""), "");
}

#[test]
fn percent_encode_hex_uppercase() {
    // %20 et non %20 en lowercase
    let out = percent_encode("?=&");
    assert!(out.chars().filter(|c| c.is_ascii_hexdigit()).all(|c| !c.is_lowercase()));
}

#[test]
fn redirect_to_returns_302_with_location() {
    let resp = redirect_to("https://example.com/path");
    assert_eq!(resp.status(), StatusCode::FOUND);
    assert_eq!(
        resp.headers().get(header::LOCATION).unwrap(),
        "https://example.com/path"
    );
}

#[test]
fn redirect_to_invalid_header_returns_500() {
    // Un header avec un caractere de controle (\n) est invalide
    let resp = redirect_to("bad\nlocation");
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn front_error_redirect_builds_login_url_with_encoded_reason() {
    let resp = front_error_redirect("https://front.example/", "state mismatch");
    assert_eq!(resp.status(), StatusCode::FOUND);
    let loc = resp.headers().get(header::LOCATION).unwrap().to_str().unwrap();
    assert!(loc.starts_with("https://front.example/login?error="));
    assert!(loc.contains("state%20mismatch"));
}

#[test]
fn front_error_redirect_strips_trailing_slash() {
    let resp = front_error_redirect("https://front.example///", "oops");
    let loc = resp.headers().get(header::LOCATION).unwrap().to_str().unwrap();
    // Ne doit pas contenir "////login"
    assert!(loc.starts_with("https://front.example/login?error="));
}

// ── CallbackQuery ──

#[test]
fn callback_query_empty_all_none() {
    let q: CallbackQuery = serde_json::from_str(r#"{}"#).unwrap();
    assert!(q.code.is_none());
    assert!(q.state.is_none());
    assert!(q.error.is_none());
    assert!(q.error_description.is_none());
}

#[test]
fn callback_query_success_with_code_and_state() {
    let raw = r#"{"code":"auth-code-123","state":"csrf-state-456"}"#;
    let q: CallbackQuery = serde_json::from_str(raw).unwrap();
    assert_eq!(q.code.as_deref(), Some("auth-code-123"));
    assert_eq!(q.state.as_deref(), Some("csrf-state-456"));
    assert!(q.error.is_none());
}

#[test]
fn callback_query_error_from_discord() {
    let raw = r#"{"error":"access_denied","error_description":"User cancelled"}"#;
    let q: CallbackQuery = serde_json::from_str(raw).unwrap();
    assert_eq!(q.error.as_deref(), Some("access_denied"));
    assert_eq!(q.error_description.as_deref(), Some("User cancelled"));
}

// ── percent_encode edge cases ──

#[test]
fn percent_encode_utf8_multibyte() {
    // é = 2 bytes UTF-8
    let enc = percent_encode("café");
    assert!(enc.contains("%C3%A9"));
}

#[test]
fn percent_encode_plus_sign() {
    // + n'est pas unreserved → encode
    assert_eq!(percent_encode("a+b"), "a%2Bb");
}

#[test]
fn percent_encode_question_mark() {
    assert_eq!(percent_encode("?"), "%3F");
}

#[test]
fn front_error_redirect_complex_error_reason() {
    let resp = front_error_redirect("https://front.example", "invalid state: expected ABC got XYZ");
    let loc = resp.headers().get(header::LOCATION).unwrap().to_str().unwrap();
    // L'espace et le colon doivent etre encodes
    assert!(loc.contains("%20"));
    assert!(loc.contains("%3A"));
}

#[test]
fn redirect_to_preserves_query_string() {
    let resp = redirect_to("/path?foo=bar&baz=qux");
    let loc = resp.headers().get(header::LOCATION).unwrap().to_str().unwrap();
    assert_eq!(loc, "/path?foo=bar&baz=qux");
}

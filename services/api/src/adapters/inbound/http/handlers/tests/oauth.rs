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

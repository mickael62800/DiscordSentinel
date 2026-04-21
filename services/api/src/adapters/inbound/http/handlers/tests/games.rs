use super::*;

#[test]
fn slugify_lowercases_ascii() {
    assert_eq!(slugify_emoji_name("HelloWorld"), "helloworld");
}

#[test]
fn slugify_replaces_whitespace_with_underscore() {
    assert_eq!(slugify_emoji_name("hello world"), "hello_world");
}

#[test]
fn slugify_collapses_multiple_separators() {
    // "a . b" → "a_b" (pas "a___b")
    assert_eq!(slugify_emoji_name("a . b"), "a_b");
}

#[test]
fn slugify_strips_trailing_and_leading_underscores() {
    assert_eq!(slugify_emoji_name("   hello   "), "hello");
}

#[test]
fn slugify_truncates_to_32_chars() {
    let long = "a".repeat(50);
    let out = slugify_emoji_name(&long);
    assert_eq!(out.len(), 32);
}

#[test]
fn slugify_pads_short_to_min_2() {
    assert_eq!(slugify_emoji_name("a").len(), 2);
    assert!(slugify_emoji_name("a").starts_with("a"));
}

#[test]
fn slugify_empty_input_produces_valid_length() {
    let out = slugify_emoji_name("");
    assert!(out.len() >= 2);
}

#[test]
fn slugify_preserves_underscores_explicitly() {
    assert_eq!(slugify_emoji_name("hello_world"), "hello_world");
}

#[test]
fn slugify_strips_non_ascii() {
    // Les emojis ne sont pas alphanumeriques ASCII → supprimes
    assert_eq!(slugify_emoji_name("cool😎name"), "coolname");
}

#[test]
fn slugify_dash_becomes_underscore() {
    assert_eq!(slugify_emoji_name("a-b-c"), "a_b_c");
}

#[test]
fn slugify_digits_preserved() {
    assert_eq!(slugify_emoji_name("Game123"), "game123");
}

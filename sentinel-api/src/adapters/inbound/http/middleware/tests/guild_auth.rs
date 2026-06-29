use super::*;

#[test]
fn extract_guild_id_finds_snowflake_in_path() {
    assert_eq!(
        extract_guild_id_from_path("/api/coude/123456789012345678/players"),
        Some("123456789012345678".to_string())
    );
}

#[test]
fn extract_guild_id_handles_no_guild() {
    assert_eq!(extract_guild_id_from_path("/api/health"), None);
    assert_eq!(extract_guild_id_from_path("/api/coude/guilds"), None);
}

#[test]
fn extract_guild_id_ignores_short_segments() {
    // /api/v10/foo : aucun segment de 17-20 chiffres
    assert_eq!(extract_guild_id_from_path("/api/v10/foo"), None);
}

#[test]
fn extract_guild_id_ignores_uuid_segments() {
    // UUID = 36 chars avec tirets, ne match pas le filtre
    assert_eq!(
        extract_guild_id_from_path("/api/coude/abcd-1234-ef56/x"),
        None
    );
}

#[test]
fn short_hash_is_stable() {
    assert_eq!(short_hash("token123"), short_hash("token123"));
    assert_ne!(short_hash("token123"), short_hash("token124"));
}

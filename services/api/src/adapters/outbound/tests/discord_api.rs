use super::*;

#[test]
fn is_configured_true_for_non_empty_token() {
    let svc = DiscordApiService::new("abc123".into());
    assert!(svc.is_configured());
}

#[test]
fn is_configured_false_for_empty_token() {
    let svc = DiscordApiService::new(String::new());
    assert!(!svc.is_configured());
}

#[test]
fn ensure_configured_returns_internal_error_when_empty() {
    let svc = DiscordApiService::new(String::new());
    let err = svc.ensure_configured().unwrap_err();
    match err {
        DomainError::Internal(msg) => assert!(msg.contains("SENTINEL_DISCORD_TOKEN")),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn ensure_configured_ok_when_token_set() {
    let svc = DiscordApiService::new("t".into());
    assert!(svc.ensure_configured().is_ok());
}

#[test]
fn avatar_url_some_when_hash_provided() {
    let url = discord_avatar_url("1234567890", Some("abc")).unwrap();
    assert_eq!(
        url,
        "https://cdn.discordapp.com/avatars/1234567890/abc.png?size=64"
    );
}

#[test]
fn avatar_url_none_when_hash_missing() {
    assert!(discord_avatar_url("123", None).is_none());
}

#[test]
fn avatar_url_handles_animated_hash() {
    // Discord utilise le prefixe "a_" pour les GIF animes — helper le conserve tel quel.
    let url = discord_avatar_url("42", Some("a_deadbeef")).unwrap();
    assert!(url.contains("/42/a_deadbeef.png"));
}

#[test]
fn user_guild_deserializes_only_id() {
    let raw = serde_json::json!({"id": "g1", "name": "My Guild", "icon": null, "permissions": "0"});
    let g: UserGuild = serde_json::from_value(raw).unwrap();
    assert_eq!(g.id, "g1");
}

#[test]
fn discord_user_default_avatar_absent() {
    let raw = serde_json::json!({"id": "u", "username": "alice"});
    let u: DiscordUser = serde_json::from_value(raw).unwrap();
    assert_eq!(u.id, "u");
    assert_eq!(u.username, "alice");
    assert!(u.avatar.is_none());
}

#[test]
fn discord_user_with_avatar() {
    let raw = serde_json::json!({"id": "u", "username": "alice", "avatar": "hash"});
    let u: DiscordUser = serde_json::from_value(raw).unwrap();
    assert_eq!(u.avatar.as_deref(), Some("hash"));
}

#[test]
fn discord_channel_deserializes_required_fields() {
    let raw = serde_json::json!({"id": "c1", "name": "general", "position": 3});
    let c: DiscordChannel = serde_json::from_value(raw).unwrap();
    assert_eq!(c.id, "c1");
    assert_eq!(c.name, "general");
    assert_eq!(c.position, 3);
}

#[test]
fn discord_member_roundtrip_json() {
    let m = DiscordMember {
        id: "u".into(),
        username: "alice".into(),
        display_name: Some("Alice".into()),
        avatar_url: Some("https://example/x.png".into()),
    };
    let json = serde_json::to_value(&m).unwrap();
    let back: DiscordMember = serde_json::from_value(json).unwrap();
    assert_eq!(back.id, "u");
    assert_eq!(back.display_name.as_deref(), Some("Alice"));
}

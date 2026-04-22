use super::*;

#[test]
fn display_name_max_is_100() {
    assert_eq!(RBAC_DISPLAY_NAME_MAX, 100);
}

// ── is_owner_self_demotion ──

#[test]
fn self_demotion_detected_when_caller_and_target_match_and_role_changes() {
    assert!(is_owner_self_demotion("u1", "u1", "admin"));
    assert!(is_owner_self_demotion("u1", "u1", "moderator"));
    assert!(is_owner_self_demotion("u1", "u1", "viewer"));
}

#[test]
fn self_update_to_owner_is_not_demotion() {
    // Un owner peut se "reattribuer" owner (no-op).
    assert!(!is_owner_self_demotion("u1", "u1", "owner"));
}

#[test]
fn update_of_another_user_is_never_self_demotion() {
    assert!(!is_owner_self_demotion("u1", "u2", "viewer"));
    assert!(!is_owner_self_demotion("u1", "u2", "admin"));
}

// ── would_revoke_last_owner ──

#[test]
fn revoke_last_owner_blocks_when_target_is_sole_owner() {
    assert!(would_revoke_last_owner(true, 1));
    // Defensif : meme 0 (ne devrait jamais arriver) doit bloquer.
    assert!(would_revoke_last_owner(true, 0));
}

#[test]
fn revoke_owner_allowed_when_others_remain() {
    assert!(!would_revoke_last_owner(true, 2));
    assert!(!would_revoke_last_owner(true, 5));
}

#[test]
fn revoke_non_owner_never_blocks() {
    // Si la cible n'est pas owner, le compte d'owners restants est sans effet.
    assert!(!would_revoke_last_owner(false, 1));
    assert!(!would_revoke_last_owner(false, 0));
}

// ── truncate_display_name ──

#[test]
fn truncate_short_name_unchanged() {
    assert_eq!(truncate_display_name("alice"), "alice");
}

#[test]
fn truncate_at_exactly_100_chars() {
    let s = "a".repeat(100);
    assert_eq!(truncate_display_name(&s).chars().count(), 100);
}

#[test]
fn truncate_over_100_chars_chops_to_100() {
    let s = "a".repeat(150);
    let out = truncate_display_name(&s);
    assert_eq!(out.chars().count(), 100);
}

#[test]
fn truncate_unicode_safe() {
    // 150 emojis (multi-byte) -> doit tenir sur 100 chars sans casser un codepoint.
    let s: String = "\u{1F600}".repeat(150);
    let out = truncate_display_name(&s);
    assert_eq!(out.chars().count(), 100);
    // Sanity : chaque char decode proprement (pas de byte orphelin).
    assert!(out.chars().all(|c| c == '\u{1F600}'));
}

#[test]
fn truncate_empty_stays_empty() {
    assert_eq!(truncate_display_name(""), "");
}

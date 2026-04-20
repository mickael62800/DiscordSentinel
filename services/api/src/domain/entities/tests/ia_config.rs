use super::*;

#[test]
fn default_for_guild_sets_all_fields() {
    let c = IaConfig::default_for_guild("my-guild");
    assert_eq!(c.guild_id, "my-guild");
    assert!(c.text_enabled);
    assert_eq!(c.text_threshold, 0.5);
    assert!(c.vision_enabled);
    assert_eq!(c.vision_threshold, 0.5);
    assert_eq!(c.context_dampening, 0.65);
    assert_eq!(c.context_format, "natural");
    assert_eq!(c.context_max_messages, 3);
    assert_eq!(c.context_max_chars, 200);
}

#[test]
fn default_created_at_equals_updated_at() {
    // Garde-fou contre la regression du pattern double-Utc::now()
    // (cf strikes/notes/moderation_service fixes).
    let c = IaConfig::default_for_guild("g");
    assert_eq!(c.created_at, c.updated_at);
}

#[test]
fn default_thresholds_are_sensible() {
    // Invariants : seuils dans [0,1], dampening dans [0,1].
    let c = IaConfig::default_for_guild("g");
    assert!((0.0..=1.0).contains(&c.text_threshold));
    assert!((0.0..=1.0).contains(&c.vision_threshold));
    assert!((0.0..=1.0).contains(&c.context_dampening));
}

#[test]
fn default_context_format_valid() {
    // "natural" ou "tagged" sont les deux valeurs valides cote consumer.
    let c = IaConfig::default_for_guild("g");
    assert!(c.context_format == "natural" || c.context_format == "tagged");
}

#[test]
fn default_context_limits_positive() {
    let c = IaConfig::default_for_guild("g");
    assert!(c.context_max_messages > 0);
    assert!(c.context_max_chars > 0);
}

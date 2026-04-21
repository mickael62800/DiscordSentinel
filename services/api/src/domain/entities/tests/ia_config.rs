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

// ── new_normalized : clamping + fallback context_format ──

#[test]
fn new_normalized_clamps_thresholds_to_0_1() {
    let c = IaConfig::new_normalized(
        "g".into(), true, -0.5, true, 1.5, 2.0, "natural".into(), 3, 200,
    );
    assert_eq!(c.text_threshold, 0.0);     // clamp lower
    assert_eq!(c.vision_threshold, 1.0);    // clamp upper
    assert_eq!(c.context_dampening, 1.0);   // clamp upper
}

#[test]
fn new_normalized_clamps_context_max_messages_0_10() {
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "natural".into(), 99, 200,
    );
    assert_eq!(c.context_max_messages, 10);
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "natural".into(), -5, 200,
    );
    assert_eq!(c.context_max_messages, 0);
}

#[test]
fn new_normalized_clamps_context_max_chars_50_500() {
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "natural".into(), 3, 10_000,
    );
    assert_eq!(c.context_max_chars, 500);
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "natural".into(), 3, 0,
    );
    assert_eq!(c.context_max_chars, 50);
}

#[test]
fn new_normalized_context_format_natural_and_tagged_accepted() {
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "natural".into(), 3, 200,
    );
    assert_eq!(c.context_format, "natural");
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "tagged".into(), 3, 200,
    );
    assert_eq!(c.context_format, "tagged");
}

#[test]
fn new_normalized_context_format_unknown_falls_back_to_natural() {
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "unknown".into(), 3, 200,
    );
    assert_eq!(c.context_format, "natural");
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "".into(), 3, 200,
    );
    assert_eq!(c.context_format, "natural");
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "NATURAL".into(), 3, 200,
    );
    assert_eq!(c.context_format, "natural"); // case-sensitive → fallback
}

#[test]
fn new_normalized_preserves_guild_id_and_booleans() {
    let c = IaConfig::new_normalized(
        "my_guild".into(), false, 0.5, true, 0.5, 0.5, "natural".into(), 3, 200,
    );
    assert_eq!(c.guild_id, "my_guild");
    assert!(!c.text_enabled);
    assert!(c.vision_enabled);
}

#[test]
fn new_normalized_created_equals_updated() {
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.5, true, 0.5, 0.5, "natural".into(), 3, 200,
    );
    // Garde-fou contre la regression du pattern double-Utc::now().
    assert_eq!(c.created_at, c.updated_at);
}

#[test]
fn new_normalized_valid_inputs_passthrough() {
    let c = IaConfig::new_normalized(
        "g".into(), true, 0.75, false, 0.3, 0.8, "tagged".into(), 5, 300,
    );
    assert_eq!(c.text_threshold, 0.75);
    assert_eq!(c.vision_threshold, 0.3);
    assert_eq!(c.context_dampening, 0.8);
    assert_eq!(c.context_format, "tagged");
    assert_eq!(c.context_max_messages, 5);
    assert_eq!(c.context_max_chars, 300);
}
